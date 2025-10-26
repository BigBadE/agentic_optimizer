//! Unified test runner.
//!
//! This module provides the test runner that executes unified test fixtures
//! with pattern-based mock LLM responses.

use super::fixture::{MatchType, TestEvent, TestFixture, TriggerConfig};
use super::verifier::{UnifiedVerifier, VerificationResult};
use async_trait::async_trait;
use merlin_core::{Context, ModelProvider, Query, Response, Result, RoutingError, TokenUsage};
use merlin_tooling::{
    BashTool, ContextRequestTool, DeleteFileTool, EditFileTool, ListFilesTool, ReadFileTool,
    ToolResult, TypeScriptRuntime, WriteFileTool,
};
use regex::Regex;
use serde_json::{Value, from_str};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tempfile::TempDir;
use tokio::time::{Duration, sleep};

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

/// UI state for testing
#[derive(Debug, Clone, Default)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "Test state struct tracking multiple boolean UI states"
)]
pub struct UiState {
    /// Current input text
    pub input_text: String,
    /// Cursor position in input
    pub cursor_position: usize,
    /// Focused pane (input, output, tasks, threads)
    pub focused_pane: String,
    /// Last output text
    pub last_output: String,
    /// Number of tasks displayed
    pub tasks_displayed: usize,
    /// Current task status
    pub task_status: Option<String>,
    /// Whether task tree is expanded
    pub task_tree_expanded: bool,
    /// Visible task descriptions
    pub task_descriptions: Vec<String>,
    /// Current progress percentage (if showing progress)
    pub progress_percentage: Option<u8>,
    /// Whether placeholder is visible
    pub placeholder_visible: bool,
    /// Task counts by status
    pub pending_count: usize,
    /// Running tasks count
    pub running_count: usize,
    /// Completed tasks count
    pub completed_count: usize,
    /// Failed tasks count
    pub failed_count: usize,
    /// Selected task description
    pub selected_task_description: Option<String>,
    /// Thread-specific state
    /// Number of active threads
    pub thread_count: usize,
    /// Currently selected thread ID (if any)
    pub selected_thread_id: Option<String>,
    /// Thread list visible (side-by-side mode active)
    pub thread_list_visible: bool,
    /// Thread names currently visible
    pub thread_names: Vec<String>,
    /// Thread colors (emojis) currently visible
    pub thread_colors: Vec<String>,
    /// Thread message counts
    pub thread_message_counts: Vec<usize>,
    /// Whether queued input prompt is showing
    pub queued_input_prompt_visible: bool,
    /// Queued input text (if any)
    pub queued_input_text: Option<String>,
    /// Whether cancel is requested
    pub cancel_requested: bool,
}

/// Test state for tracking execution
#[derive(Debug, Clone, Default)]
pub struct TestState {
    /// Conversation message count
    pub conversation_count: usize,
    /// Currently selected task ID
    pub selected_task: Option<String>,
    /// Vector cache status
    pub vector_cache_status: Option<String>,
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
    /// UI state
    ui_state: Arc<Mutex<UiState>>,
    /// Test state
    test_state: Arc<Mutex<TestState>>,
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

        Ok(Self {
            fixture,
            workspace,
            provider,
            runtime,
            ui_state: Arc::new(Mutex::new(UiState {
                focused_pane: "input".to_owned(),
                ..Default::default()
            })),
            test_state: Arc::new(Mutex::new(TestState::default())),
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

    /// Handle user input event
    ///
    /// # Errors
    /// Returns error if state lock fails
    fn handle_user_input(&self, input_event: &super::fixture::UserInputEvent) -> Result<()> {
        // Update UI state
        {
            let mut ui = self
                .ui_state
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;

            // Replace text (each user_input event sets the complete input text)
            ui.input_text.clone_from(&input_event.data.text);
            ui.cursor_position = ui.input_text.len();

            if input_event.data.submit {
                ui.input_text.clear();
                ui.cursor_position = 0;
                // Change focus to output pane after submit
                "output".clone_into(&mut ui.focused_pane);
            } else {
                // Not submitting, keep focus on input
                "input".clone_into(&mut ui.focused_pane);
            }
        }

        // Increment conversation count if submitting
        if input_event.data.submit {
            let mut state = self
                .test_state
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;
            state.conversation_count += 1;
        }

        Ok(())
    }

    /// Handle key press event
    ///
    /// # Errors
    /// Returns error if state lock fails
    fn handle_key_press(&self, key_event: &super::fixture::KeyPressEvent) -> Result<()> {
        {
            let mut ui = self
                .ui_state
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;

            match key_event.data.key.as_str() {
                "Tab" => {
                    // Cycle through panes: input -> output -> tasks -> input
                    match ui.focused_pane.as_str() {
                        "input" => "output".clone_into(&mut ui.focused_pane),
                        "output" => "tasks".clone_into(&mut ui.focused_pane),
                        _ => "input".clone_into(&mut ui.focused_pane),
                    }
                }
                "Up" | "Down" | "PageUp" | "PageDown" => {
                    // Navigation keys don't change focus, just update focused pane to tasks
                    if ui.focused_pane != "tasks" {
                        "tasks".clone_into(&mut ui.focused_pane);
                    }
                }
                _ => {
                    // Other keys don't affect state
                }
            }
        }

        Ok(())
    }

    /// Handle LLM response event
    ///
    /// # Errors
    /// Returns error if execution or state update fails
    async fn handle_llm_response(
        &self,
        llm_event: &super::fixture::LlmResponseEvent,
    ) -> Result<ToolResult<Value>> {
        // Increment conversation count
        {
            let mut state = self
                .test_state
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;
            state.conversation_count += 1;
        }

        // Execute TypeScript
        let typescript = llm_event.response.typescript.join("\n");
        let execution_result = self.runtime.execute(&typescript).await;

        // Update UI state
        if let Ok(ref value) = execution_result {
            let output_str = format!("{value}");
            let mut ui = self
                .ui_state
                .lock()
                .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?;
            ui.last_output = output_str;
            "output".clone_into(&mut ui.focused_pane);
            ui.tasks_displayed += 1;
            ui.task_status = Some("completed".to_owned());
            ui.completed_count += 1;
        }

        Ok(execution_result)
    }

    /// Run the test
    ///
    /// # Errors
    /// Returns error if test execution or verification fails
    pub async fn run(&self) -> Result<VerificationResult> {
        let mut verifier = UnifiedVerifier::new(&self.fixture, self.workspace.path());

        // Process each event
        for event in &self.fixture.events {
            match event {
                TestEvent::UserInput(input_event) => {
                    self.handle_user_input(input_event)?;

                    // Pass state to verifier
                    let ui_state = self
                        .ui_state
                        .lock()
                        .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
                        .clone();
                    let test_state = self
                        .test_state
                        .lock()
                        .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
                        .clone();

                    verifier.set_ui_state(ui_state);
                    verifier.set_test_state(test_state);

                    verifier
                        .verify_event(event, &input_event.verify)
                        .map_err(RoutingError::ExecutionFailed)?;
                }
                TestEvent::KeyPress(key_event) => {
                    self.handle_key_press(key_event)?;

                    // Pass state to verifier
                    let ui_state = self
                        .ui_state
                        .lock()
                        .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
                        .clone();
                    let test_state = self
                        .test_state
                        .lock()
                        .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
                        .clone();

                    verifier.set_ui_state(ui_state);
                    verifier.set_test_state(test_state);

                    verifier
                        .verify_event(event, &key_event.verify)
                        .map_err(RoutingError::ExecutionFailed)?;
                }
                TestEvent::LlmResponse(llm_event) => {
                    let execution_result = self.handle_llm_response(llm_event).await?;

                    verifier.set_last_execution_result(execution_result);

                    // Pass state to verifier
                    let ui_state = self
                        .ui_state
                        .lock()
                        .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
                        .clone();
                    let test_state = self
                        .test_state
                        .lock()
                        .map_err(|err| RoutingError::Other(format!("Lock error: {err}")))?
                        .clone();

                    verifier.set_ui_state(ui_state);
                    verifier.set_test_state(test_state);

                    verifier
                        .verify_event(event, &llm_event.verify)
                        .map_err(RoutingError::ExecutionFailed)?;
                }
                TestEvent::Wait(wait_event) => {
                    // Sleep for specified duration
                    sleep(Duration::from_millis(wait_event.data.duration_ms)).await;
                }
            }
        }

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
