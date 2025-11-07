//! Test runner setup and initialization logic.

use crate::event_source::{FixtureEventController, FixtureEventSource};
use crate::fixture::{TestEvent, TestFixture};
use crate::mock_provider::{MockProvider, MockRouter};
use crate::tui_test_helpers;
use crate::workspace_setup::{create_files, get_test_workspace_path};
use merlin_agent::{RoutingOrchestrator, ThreadStore};
use merlin_cli::TuiApp;
use merlin_core::{ModelProvider, Result, RoutingError};
use merlin_deps::ratatui::backend::TestBackend;
use merlin_deps::tempfile::TempDir;
use merlin_routing::{Model, ProviderRegistry, RoutingConfig};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Components needed to construct a test runner
pub struct RunnerComponents {
    /// Temporary workspace (for writable tests, auto-cleanup)
    pub workspace_temp: Option<TempDir>,
    /// Workspace path
    pub workspace_path: PathBuf,
    /// Mock provider
    pub provider: Arc<MockProvider>,
    /// TUI application
    pub tui_app: TuiApp<TestBackend>,
    /// Event controller
    pub event_controller: FixtureEventController,
}

/// Create test runner components
///
/// Context fixtures use read-only workspaces with pre-generated embeddings.
/// Other fixtures use temporary writable workspaces (created only when needed).
///
/// # Errors
/// Returns error if setup fails
pub fn create_runner_components(fixture: &TestFixture) -> Result<RunnerComponents> {
    let provider = Arc::new(MockProvider::new("test-mock"));

    // Determine final workspace path
    // Fixtures with explicit workspace use read-only pre-made workspaces (e.g. context tests)
    // All other fixtures get temp workspaces (may write files at runtime)
    let (final_workspace_path, workspace_temp) = if let Some(ws_name) = &fixture.setup.workspace {
        // Use pre-made read-only workspace (for context tests with embeddings)
        (get_test_workspace_path(ws_name)?, None)
    } else {
        // Create temp workspace - fixture may write files at runtime
        let workspace = TempDir::new()
            .map_err(|err| RoutingError::Other(format!("Failed to create workspace: {err}")))?;
        let workspace_path = workspace.path().to_path_buf();

        // Create pre-specified files if any
        if !fixture.setup.files.is_empty() {
            create_files(&workspace_path, &fixture.setup.files)?;
        }

        (workspace_path, Some(workspace))
    };

    // Setup LLM response patterns
    for event in &fixture.events {
        if let TestEvent::LlmResponse(llm_event) = event {
            let typescript = llm_event.as_ref().response.typescript.join("\n");
            provider.add_response(&llm_event.as_ref().trigger, typescript)?;
        }
    }

    // Create routing config for test orchestrator
    let mut config = RoutingConfig::default();
    // Disable all real tiers
    config.tiers.local_enabled = false;
    config.tiers.groq_enabled = false;
    config.tiers.premium_enabled = false;

    // Create provider registry and register mock provider
    let mut registry = ProviderRegistry::new(config.clone())?;
    registry.register_provider(
        Model::Qwen25Coder32B,
        Arc::clone(&provider) as Arc<dyn ModelProvider>,
    );

    // Create orchestrator with mock router and provider registry
    let router = Arc::new(MockRouter::new());

    // Determine if embeddings should be enabled
    // Only enable for fixtures that use pre-made test workspaces (which have cached embeddings)
    let enable_embeddings = fixture.setup.workspace.is_some();

    // Create thread store for conversation management if fixture uses threads
    let needs_threads = fixture.tags.contains(&"threads".to_owned());
    let orchestrator = if needs_threads {
        let thread_storage_path = final_workspace_path.join(".merlin").join("threads");
        let store = ThreadStore::new(thread_storage_path)?;
        let thread_store = Arc::new(Mutex::new(store));
        RoutingOrchestrator::new_with_router(config, router, Arc::new(registry))?
            .with_workspace(final_workspace_path.clone())
            .with_embeddings(enable_embeddings)
            .with_thread_store(thread_store)
    } else {
        RoutingOrchestrator::new_with_router(config, router, Arc::new(registry))?
            .with_workspace(final_workspace_path.clone())
            .with_embeddings(enable_embeddings)
    };

    // Create fixture-based event source with controller
    let (event_source, event_controller) = FixtureEventSource::new(fixture);

    // Create test backend with reasonable size
    let terminal_size = fixture.setup.terminal_size.unwrap_or((80, 24));
    let backend = TestBackend::new(terminal_size.0, terminal_size.1);

    // Create TUI app with test backend, fixture event source, and orchestrator
    let tui_app = tui_test_helpers::new_test_app(
        backend,
        Box::new(event_source),
        Some(final_workspace_path.clone()),
        Some(Arc::new(orchestrator)),
    )?;

    Ok(RunnerComponents {
        workspace_temp,
        workspace_path: final_workspace_path,
        provider,
        tui_app,
        event_controller,
    })
}
