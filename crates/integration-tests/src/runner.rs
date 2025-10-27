//! Unified test runner.
//!
//! This module provides the test runner that executes unified test fixtures
//! by running the actual CLI with pattern-based mock LLM responses.

use super::event_source::FixtureEventSource;
use super::fixture::{MatchType, TestEvent, TestFixture, TriggerConfig};
use super::verification_result::VerificationResult;
use super::verifier::UnifiedVerifier;
use async_trait::async_trait;
use merlin_cli::TuiApp;
use merlin_core::{Context, ModelProvider, Query, Response, Result, RoutingError, TokenUsage};
use merlin_tooling::{
    BashTool, ContextRequestTool, DeleteFileTool, EditFileTool, ListFilesTool, ReadFileTool,
    TypeScriptRuntime, WriteFileTool,
};
use ratatui::backend::TestBackend;
use regex::Regex;
use serde_json::from_str;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tempfile::TempDir;
use tokio::time::{Duration as TokioDuration, sleep};

/// Pattern-based mock provider
pub struct PatternMockProvider {
    /// Provider name
    name: &'static str,
    /// Pattern responses
    responses: Arc<Mutex<Vec<PatternResponse>>>,
    /// Call counter for debugging
    call_count: Arc<AtomicUsize>,
}

/// Pattern response configuration
struct PatternResponse {
    /// Trigger pattern
    pattern: String,
    /// Match type
    match_type: MatchType,
    /// TypeScript response
    typescript: String,
    /// Whether this response has been used
    used: bool,
    /// Compiled regex (if `match_type` is Regex)
    regex: Option<Regex>,
}

impl PatternResponse {
    /// Create new pattern response
    ///
    /// # Errors
    /// Returns error if regex compilation fails
    fn new(trigger: &TriggerConfig, typescript: String) -> Result<Self> {
        let regex = matches!(trigger.match_type, MatchType::Regex)
            .then(|| {
                Regex::new(&trigger.pattern)
                    .map_err(|err| RoutingError::InvalidTask(format!("Invalid regex: {err}")))
            })
            .transpose()?;

        Ok(Self {
            pattern: trigger.pattern.clone(),
            match_type: trigger.match_type,
            typescript,
            used: false,
            regex,
        })
    }

    /// Check if this pattern matches the query
    fn matches(&self, query_text: &str) -> bool {
        match self.match_type {
            MatchType::Exact => query_text == self.pattern,
            MatchType::Contains => query_text.contains(&self.pattern),
            MatchType::Regex => self
                .regex
                .as_ref()
                .is_some_and(|regex| regex.is_match(query_text)),
        }
    }
}

impl PatternMockProvider {
    /// Create new pattern mock provider
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            responses: Arc::new(Mutex::new(Vec::new())),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Add response pattern
    ///
    /// # Errors
    /// Returns error if pattern is invalid
    pub fn add_response(&self, trigger: &TriggerConfig, typescript: String) -> Result<()> {
        let response = PatternResponse::new(trigger, typescript)?;
        self.responses
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
            .push(response);
        Ok(())
    }

    /// Get matching response for query
    ///
    /// # Errors
    /// Returns error if no matching pattern found
    fn get_response(&self, query_text: &str) -> Result<String> {
        let mut responses = self
            .responses
            .lock()
            .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;

        // Find first unused matching pattern
        let result = responses
            .iter_mut()
            .find(|resp| !resp.used && resp.matches(query_text))
            .map(|resp| {
                resp.used = true;
                resp.typescript.clone()
            });

        drop(responses);

        result.ok_or_else(|| {
            RoutingError::ExecutionFailed(format!("No matching pattern for query: {query_text}"))
        })
    }

    /// Reset all patterns to unused (for testing)
    ///
    /// # Errors
    /// Returns error if lock acquisition fails
    pub fn reset(&self) -> Result<()> {
        {
            let mut responses = self
                .responses
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;
            for response in responses.iter_mut() {
                response.used = false;
            }
        }
        self.call_count.store(0, Ordering::SeqCst);
        Ok(())
    }
}

#[async_trait]
impl ModelProvider for PatternMockProvider {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn is_available(&self) -> bool {
        true
    }

    async fn generate(&self, query: &Query, _context: &Context) -> Result<Response> {
        // Increment call count
        self.call_count.fetch_add(1, Ordering::SeqCst);

        // Get matching response
        let typescript = self.get_response(&query.text)?;

        // Wrap TypeScript in code block
        let content = format!("```typescript\n{typescript}\n```");

        Ok(Response {
            text: content,
            confidence: 1.0,
            tokens_used: TokenUsage {
                input: query.text.len() as u64,
                output: typescript.len() as u64,
                cache_read: 0,
                cache_write: 0,
            },
            provider: self.name.to_owned(),
            latency_ms: 0,
        })
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0 // Mock provider is free
    }
}

/// Unified test runner
pub struct UnifiedTestRunner {
    /// Test fixture
    fixture: TestFixture,
    /// Workspace directory
    workspace: TempDir,
    /// Mock provider
    provider: Arc<PatternMockProvider>,
    /// TypeScript runtime for executing LLM responses
    runtime: TypeScriptRuntime,
    /// The actual TUI application under test
    tui_app: TuiApp<TestBackend>,
}

impl UnifiedTestRunner {
    /// Create new test runner
    ///
    /// # Errors
    /// Returns error if workspace setup fails
    pub fn new(fixture: TestFixture) -> Result<Self> {
        let workspace = TempDir::new()
            .map_err(|err| RoutingError::Other(format!("Failed to create workspace: {err}")))?;

        let provider = Arc::new(PatternMockProvider::new("test-mock"));

        // Setup files
        for (path, content) in &fixture.setup.files {
            let file_path = workspace.path().join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    RoutingError::Other(format!("Failed to create directory: {err}"))
                })?;
            }
            fs::write(&file_path, content)
                .map_err(|err| RoutingError::Other(format!("Failed to write file: {err}")))?;
        }

        // Setup LLM response patterns
        for event in &fixture.events {
            if let TestEvent::LlmResponse(llm_event) = event {
                let typescript = llm_event.response.typescript.join("\n");
                provider.add_response(&llm_event.trigger, typescript)?;
            }
        }

        // Setup TypeScript runtime with file operation tools
        let mut runtime = TypeScriptRuntime::new();
        runtime.register_tool(Arc::new(BashTool));
        runtime.register_tool(Arc::new(ContextRequestTool::new(
            workspace.path().to_path_buf(),
        )));
        runtime.register_tool(Arc::new(DeleteFileTool::new(workspace.path())));
        runtime.register_tool(Arc::new(EditFileTool::new(workspace.path())));
        runtime.register_tool(Arc::new(WriteFileTool::new(workspace.path())));
        runtime.register_tool(Arc::new(ReadFileTool::new(workspace.path())));
        runtime.register_tool(Arc::new(ListFilesTool::new(workspace.path())));

        // Create fixture-based event source
        let event_source = Box::new(FixtureEventSource::new(&fixture));

        // Create test backend with reasonable size
        let terminal_size = fixture.setup.terminal_size.unwrap_or((80, 24));
        let backend = TestBackend::new(terminal_size.0, terminal_size.1);

        // Create TUI app with test backend and fixture event source
        let (tui_app, _ui_channel) =
            TuiApp::new_for_test(backend, event_source, Some(workspace.path().to_path_buf()))?;

        Ok(Self {
            fixture,
            workspace,
            provider,
            runtime,
            tui_app,
        })
    }

    /// Get workspace path
    #[must_use]
    pub fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }

    /// Get mock provider
    #[must_use]
    pub fn provider(&self) -> Arc<PatternMockProvider> {
        Arc::clone(&self.provider)
    }

    /// Get read-only reference to TUI app for verification
    #[must_use]
    pub fn tui_app(&self) -> &TuiApp<TestBackend> {
        &self.tui_app
    }

    /// Run the test
    ///
    /// # Errors
    /// Returns error if test execution or verification fails
    pub async fn run(&mut self) -> Result<VerificationResult> {
        let mut verifier = UnifiedVerifier::new(&self.fixture, self.workspace.path());

        // Process each event in the fixture
        for event in &self.fixture.events {
            match event {
                TestEvent::UserInput(_) | TestEvent::KeyPress(_) => {
                    // These events are already loaded into the FixtureEventSource
                    // Drive the TUI to process one event from the queue
                    let _should_quit = self.tui_app.tick()?;

                    // Process any UI events that were triggered
                    while self.tui_app.tick()? {
                        // Keep processing until no more events
                    }
                }
                TestEvent::LlmResponse(llm_event) => {
                    // Execute the TypeScript response
                    let typescript = llm_event.response.typescript.join("\n");
                    let execution_result = self.runtime.execute(&typescript).await;

                    verifier.set_last_execution_result(execution_result);

                    // Verify this event
                    verifier
                        .verify_event(event, &llm_event.verify)
                        .map_err(RoutingError::ExecutionFailed)?;

                    // Let TUI process any resulting UI events
                    self.tui_app.tick()?;
                }
                TestEvent::Wait(wait_event) => {
                    // Sleep for specified duration
                    sleep(TokioDuration::from_millis(wait_event.data.duration_ms)).await;

                    // Process any pending UI events during wait
                    self.tui_app.tick()?;
                }
            }
        }

        // Pass TUI app state to verifier for final verification
        verifier.set_tui_app(&self.tui_app);

        // Verify final state
        verifier
            .verify_final(&self.fixture.final_verify)
            .map_err(RoutingError::ExecutionFailed)?;

        Ok(verifier.result())
    }

    /// Load fixture from file
    ///
    /// # Errors
    /// Returns error if file reading or parsing fails
    pub fn load_fixture(path: &Path) -> Result<TestFixture> {
        let content = fs::read_to_string(path)
            .map_err(|err| RoutingError::Other(format!("Failed to read fixture: {err}")))?;
        from_str(&content)
            .map_err(|err| RoutingError::Other(format!("Failed to parse fixture: {err}")))
    }

    /// Discover all fixtures in directory
    ///
    /// # Errors
    /// Returns error if directory reading fails
    pub fn discover_fixtures(dir: &Path) -> Result<Vec<PathBuf>> {
        let mut fixtures = Vec::new();

        if !dir.exists() {
            return Ok(fixtures);
        }

        let entries = fs::read_dir(dir)
            .map_err(|err| RoutingError::Other(format!("Failed to read directory: {err}")))?;

        for entry in entries {
            let entry =
                entry.map_err(|err| RoutingError::Other(format!("Failed to read entry: {err}")))?;
            let path = entry.path();

            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                fixtures.push(path);
            } else if path.is_dir() {
                // Recurse into subdirectories
                let mut sub_fixtures = Self::discover_fixtures(&path)?;
                fixtures.append(&mut sub_fixtures);
            }
        }

        Ok(fixtures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests exact pattern matching
    ///
    /// # Panics
    /// Panics if pattern creation fails
    #[test]
    #[cfg_attr(test, allow(clippy::unwrap_used, reason = "Allow for tests"))]
    fn test_pattern_response_exact_match() {
        let trigger = TriggerConfig {
            pattern: "hello world".to_owned(),
            match_type: MatchType::Exact,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned()).unwrap();
        assert!(response.matches("hello world"));
        assert!(!response.matches("hello"));
        assert!(!response.matches("hello world!"));
    }

    /// Tests contains pattern matching
    ///
    /// # Panics
    /// Panics if pattern creation fails
    #[test]
    #[cfg_attr(test, allow(clippy::unwrap_used, reason = "Allow for tests"))]
    fn test_pattern_response_contains_match() {
        let trigger = TriggerConfig {
            pattern: "world".to_owned(),
            match_type: MatchType::Contains,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned()).unwrap();
        assert!(response.matches("hello world"));
        assert!(response.matches("world"));
        assert!(response.matches("world hello"));
        assert!(!response.matches("hello"));
    }

    /// Tests regex pattern matching
    ///
    /// # Panics
    /// Panics if pattern creation fails
    #[test]
    #[cfg_attr(test, allow(clippy::unwrap_used, reason = "Allow for tests"))]
    fn test_pattern_response_regex_match() {
        let trigger = TriggerConfig {
            pattern: r"hello\s+\w+".to_owned(),
            match_type: MatchType::Regex,
        };
        let response = PatternResponse::new(&trigger, "test".to_owned()).unwrap();
        assert!(response.matches("hello world"));
        assert!(response.matches("hello there"));
        assert!(!response.matches("hello"));
        assert!(!response.matches("helloworld"));
    }
}
