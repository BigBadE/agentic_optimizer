//! E2E test runner that uses real code paths with mock providers.

use super::fixture::E2EFixture;
use super::mock_provider::StatefulMockProvider;
use super::verifier::{E2EVerifier, print_verification_result};
use merlin_agent::RoutingOrchestrator;
use merlin_core::ui::{UiChannel, UiEvent};
use merlin_core::{ModelProvider, RoutingConfig, Task, TaskResult};
use merlin_routing::{ProviderRegistry, StrategyRouter};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::spawn;
use tokio::sync::mpsc;

/// E2E test execution result
pub struct E2EExecutionResult {
    /// The task result from orchestrator
    pub task_result: TaskResult,
    /// The mock provider used (for call history verification)
    pub mock_provider: StatefulMockProvider,
    /// Workspace root directory
    pub workspace_root: PathBuf,
}

/// E2E test runner
pub struct E2ERunner {
    /// Temporary workspace directory
    workspace: TempDir,
    /// Environment variables set for this test
    env_vars_set: Vec<String>,
}

impl E2ERunner {
    /// Create a new E2E runner with a fresh workspace
    ///
    /// # Errors
    /// Returns error if workspace creation fails
    pub fn new() -> Result<Self, String> {
        let workspace = TempDir::new().map_err(|e| format!("Failed to create temp dir: {e}"))?;
        Ok(Self {
            workspace,
            env_vars_set: Vec::new(),
        })
    }

    /// Get the workspace root path
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        self.workspace.path()
    }

    /// Setup the workspace for a fixture
    ///
    /// # Errors
    /// Returns error if setup fails
    pub fn setup_workspace(&mut self, fixture: &E2EFixture) -> Result<(), String> {
        let workspace_root = self.workspace.path();

        // Create basic Rust project structure
        fs::create_dir_all(workspace_root.join("src"))
            .map_err(|e| format!("Failed to create src dir: {e}"))?;

        // Create minimal Cargo.toml
        let cargo_toml = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#;
        fs::write(workspace_root.join("Cargo.toml"), cargo_toml)
            .map_err(|e| format!("Failed to write Cargo.toml: {e}"))?;

        // Create minimal main.rs
        fs::write(workspace_root.join("src/main.rs"), "fn main() {}\n")
            .map_err(|e| format!("Failed to write main.rs: {e}"))?;

        // Setup fixture files
        for (path, content) in &fixture.setup_files {
            let file_path = workspace_root.join(path);

            // Create parent directories if needed
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent dir for {path}: {e}"))?;
            }

            fs::write(&file_path, content)
                .map_err(|e| format!("Failed to write setup file {path}: {e}"))?;
        }

        // Set environment variables
        for (key, value) in &fixture.env_vars {
            // SAFETY: Setting env vars at test setup before any concurrent access
            unsafe {
                env::set_var(key, value);
            }
            self.env_vars_set.push(key.clone());
        }

        // Set MERLIN_FOLDER to use this temp directory
        // SAFETY: Setting env var at test setup before any concurrent access
        unsafe {
            env::set_var("MERLIN_FOLDER", workspace_root.join(".merlin"));
            env::set_var("MERLIN_SKIP_EMBEDDINGS", "1");
        }
        self.env_vars_set.push("MERLIN_FOLDER".to_owned());
        self.env_vars_set.push("MERLIN_SKIP_EMBEDDINGS".to_owned());

        Ok(())
    }

    /// Create an orchestrator with mock provider (uses real production code)
    ///
    /// # Errors
    /// Returns error if orchestrator creation fails
    fn create_orchestrator(
        mock_provider: &Arc<dyn ModelProvider>,
        workspace_root: &Path,
    ) -> Result<RoutingOrchestrator, String> {
        // Create provider registry with mock provider
        let provider_registry1 = ProviderRegistry::with_mock_provider(mock_provider)
            .map_err(|e| format!("Failed to create provider registry: {e}"))?;

        // Create a second identical registry for the orchestrator (they're cheap to clone patterns)
        let provider_registry2 = ProviderRegistry::with_mock_provider(mock_provider)
            .map_err(|e| format!("Failed to create provider registry: {e}"))?;

        // Create router (same as production)
        let router = Arc::new(StrategyRouter::new(provider_registry1));

        // Create config with workspace path
        let mut config = RoutingConfig::default();
        config.workspace.root_path = workspace_root.to_path_buf();

        // Create orchestrator directly with mock router and registry (bypasses provider initialization)
        RoutingOrchestrator::new_with_router(config, router, Arc::new(provider_registry2))
            .map_err(|e| format!("Failed to create orchestrator: {e}"))
    }

    /// Execute a fixture test
    ///
    /// # Errors
    /// Returns error if execution fails
    pub async fn execute_fixture(
        &mut self,
        fixture: &E2EFixture,
    ) -> Result<E2EExecutionResult, String> {
        // Setup workspace
        self.setup_workspace(fixture)?;

        // Create mock provider from fixture responses
        let mut mock_provider = StatefulMockProvider::new("test");
        mock_provider.add_responses(fixture.mock_responses.clone());

        let mock_provider_arc = Arc::new(mock_provider.clone()) as Arc<dyn ModelProvider>;

        // Create orchestrator (uses real production code)
        let orchestrator = Self::create_orchestrator(&mock_provider_arc, self.workspace.path())?;

        // Create task and UI channel
        let task = Task::new(fixture.initial_query.clone());
        let (tx, rx) = mpsc::unbounded_channel();
        let ui_channel = UiChannel::from_sender(tx);

        // Spawn task to drain UI channel
        let _drain_handle = spawn(async move {
            Self::drain_ui_channel(rx).await;
        });

        // Execute task using orchestrator (handles everything)
        let task_result = orchestrator
            .execute_task_streaming(task, ui_channel)
            .await
            .map_err(|e| format!("Orchestrator execution failed: {e}"))?;

        Ok(E2EExecutionResult {
            task_result,
            mock_provider,
            workspace_root: self.workspace.path().to_path_buf(),
        })
    }

    /// Drain UI channel to prevent blocking
    async fn drain_ui_channel(mut rx: mpsc::UnboundedReceiver<UiEvent>) {
        while rx.recv().await.is_some() {
            // Drain events
        }
    }

    /// Run a fixture test with full verification
    ///
    /// # Errors
    /// Returns error if test execution or verification fails
    pub async fn run_fixture_test(&mut self, fixture: &E2EFixture) -> Result<(), String> {
        // Validate fixture first
        fixture.validate()?;

        // Execute the test
        let result = self.execute_fixture(fixture).await?;

        // Verify results
        let verifier = E2EVerifier::new(fixture, &result.workspace_root);
        let verification = verifier.verify_all(&result.task_result, &result.mock_provider);

        // Print verification results
        print_verification_result(&fixture.name, &verification);

        if verification.passed {
            Ok(())
        } else {
            Err(format!(
                "Verification failed with {} failures",
                verification.failures.len()
            ))
        }
    }

    /// Run all fixtures in a directory
    ///
    /// # Errors
    /// Returns error if any test fails
    pub async fn run_all_fixtures(fixtures_dir: impl AsRef<Path>) -> Result<(), String> {
        let fixtures = E2EFixture::discover_fixtures(fixtures_dir)?;

        if fixtures.is_empty() {
            return Err("No fixtures found".to_owned());
        }

        println!("\n========================================");
        println!("Running {} E2E fixtures", fixtures.len());
        println!("========================================\n");

        let mut results = HashMap::new();

        for fixture in fixtures {
            println!("Running fixture: {}", fixture.name);

            // Create a new runner for each fixture (fresh workspace)
            let mut runner = Self::new()?;

            let test_result = runner.run_fixture_test(&fixture).await;

            match test_result {
                Ok(()) => {
                    println!("✅ {}", fixture.name);
                    results.insert(fixture.name.clone(), true);
                }
                Err(e) => {
                    println!("❌ {} - {e}", fixture.name);
                    results.insert(fixture.name.clone(), false);
                }
            }
        }

        // Print summary
        let passed = results.values().filter(|&&v| v).count();
        let failed = results.len() - passed;

        println!("\n========================================");
        println!("E2E Test Summary");
        println!("========================================");
        println!("Total: {}", results.len());
        println!("Passed: {passed}");
        println!("Failed: {failed}");

        if failed > 0 {
            println!("\nFailed tests:");
            for (name, test_passed) in results {
                if !test_passed {
                    println!("  ❌ {name}");
                }
            }
            println!("========================================\n");
            return Err(format!("{failed} test(s) failed"));
        }

        println!("========================================\n");
        Ok(())
    }
}

impl Drop for E2ERunner {
    fn drop(&mut self) {
        // Clean up environment variables
        for key in &self.env_vars_set {
            // SAFETY: Cleaning up env vars set by this runner
            unsafe {
                env::remove_var(key);
            }
        }
    }
}
