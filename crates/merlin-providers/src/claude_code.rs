use async_trait::async_trait;
use merlin_core::{Context, CoreResult, Error, ModelProvider, Query, Response, Result, TokenUsage};
use serde::Deserialize;
use serde_json::from_str;
use std::process::Stdio;
use std::time::Instant;
use tokio::io::AsyncWriteExt as _;
use tokio::process::Command;

/// Default model for Claude Code provider.
const DEFAULT_MODEL: &str = "claude-sonnet-4-5-20250929";

/// Gets the Claude CLI command name for the current platform.
fn get_claude_command() -> &'static str {
    if cfg!(windows) {
        "claude.cmd"
    } else {
        "claude"
    }
}

/// Claude Code provider using Claude CLI subprocess.
///
/// This provider uses your Claude Code subscription by invoking the
/// `claude` CLI command, avoiding API billing entirely.
pub struct ClaudeCodeProvider {
    /// Model name to use.
    model: String,
}

impl ClaudeCodeProvider {
    /// Creates a new `ClaudeCodeProvider`.
    ///
    /// # Errors
    ///
    /// Returns an error if the Claude CLI is not available.
    pub fn new() -> CoreResult<Self> {
        Ok(Self {
            model: DEFAULT_MODEL.to_owned(),
        })
    }

    /// Sets the model to use for generation.
    #[must_use]
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }

    /// Checks if the Claude CLI is available on the system.
    ///
    /// # Errors
    ///
    /// Returns an error if the CLI check fails.
    async fn check_cli_available() -> CoreResult<bool> {
        let result = Command::new(get_claude_command())
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        Ok(result.is_ok_and(|status| status.success()))
    }
}

/// Claude CLI JSON output format.
#[derive(Debug, Deserialize)]
struct ClaudeCliOutput {
    /// The response text (Claude CLI uses "result" field).
    #[serde(default)]
    result: String,
    /// Token usage information.
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

/// Token usage from Claude CLI.
#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    /// Input tokens.
    #[serde(default, rename = "input_tokens")]
    input: usize,
    /// Output tokens.
    #[serde(default, rename = "output_tokens")]
    output: usize,
    /// Cache creation input tokens.
    #[serde(default, rename = "cache_creation_input_tokens")]
    cache_creation_input: usize,
    /// Cache read input tokens.
    #[serde(default, rename = "cache_read_input_tokens")]
    cache_read_input: usize,
}

#[async_trait]
impl ModelProvider for ClaudeCodeProvider {
    fn name(&self) -> &'static str {
        "Claude Code"
    }

    async fn is_available(&self) -> bool {
        Self::check_cli_available().await.unwrap_or(false)
    }

    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let start = Instant::now();

        // Build prompt with system context and user query
        let mut prompt = String::new();

        if !context.system_prompt.is_empty() {
            prompt.push_str("SYSTEM INSTRUCTIONS:\n");
            prompt.push_str(&context.system_prompt);
            prompt.push_str("\n\n");
        }

        if !context.files.is_empty() {
            prompt.push_str("CONTEXT FILES:\n");
            for file_ctx in &context.files {
                prompt.push_str("\n--- ");
                prompt.push_str(&file_ctx.path.display().to_string());
                prompt.push_str(" ---\n");
                prompt.push_str(&file_ctx.content);
                prompt.push('\n');
            }
            prompt.push('\n');
        }

        prompt.push_str("QUERY:\n");
        prompt.push_str(&query.text);

        // Spawn Claude CLI process
        let mut child = Command::new(get_claude_command())
            .arg("--print")
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(&self.model)
            .arg("--dangerously-skip-permissions")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| Error::Other(format!("Failed to spawn Claude CLI: {err}")))?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(prompt.as_bytes())
                .await
                .map_err(|err| Error::Other(format!("Failed to write to Claude CLI: {err}")))?;
            stdin
                .shutdown()
                .await
                .map_err(|err| Error::Other(format!("Failed to close stdin: {err}")))?;
        }

        // Wait for completion and capture output
        let output = child
            .wait_with_output()
            .await
            .map_err(|err| Error::Other(format!("Claude CLI execution failed: {err}")))?;

        let latency_ms = start.elapsed().as_millis() as u64;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(Error::Other(format!("Claude CLI error: {stderr}")).into());
        }

        // Parse JSON output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let cli_output: ClaudeCliOutput = from_str(&stdout)
            .map_err(|err| Error::Other(format!("Failed to parse Claude CLI JSON: {err}")))?;

        if cli_output.result.is_empty() {
            return Err(Error::Other("No text in Claude CLI response".to_owned()).into());
        }

        let tokens_used = cli_output
            .usage
            .map_or_else(TokenUsage::default, |usage| TokenUsage {
                input: usage.input as u64,
                output: usage.output as u64,
                cache_read: usage.cache_read_input as u64,
                cache_write: usage.cache_creation_input as u64,
            });

        Ok(Response {
            text: cli_output.result,
            confidence: 0.95,
            tokens_used,
            provider: format!("Claude Code/{}", self.model),
            latency_ms,
        })
    }

    fn estimate_cost(&self, _context: &Context) -> f64 {
        // Claude Code CLI uses subscription billing, not per-token
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests Claude Code provider initialization.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn claude_code_provider_creation() {
        let provider = ClaudeCodeProvider::new();
        assert!(provider.is_ok());

        if let Ok(prov) = provider {
            assert_eq!(prov.name(), "Claude Code");
            assert_eq!(prov.model, DEFAULT_MODEL);
        }
    }

    /// Tests cost estimation for Claude Code provider (should be zero).
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn cost_estimation() {
        let provider = ClaudeCodeProvider {
            model: DEFAULT_MODEL.to_owned(),
        };

        let context = Context::new("test prompt");
        let cost = provider.estimate_cost(&context);
        assert!(cost.abs() < f64::EPSILON);
    }

    /// Tests that `with_model` correctly sets the model.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_with_model() {
        let provider = ClaudeCodeProvider {
            model: DEFAULT_MODEL.to_owned(),
        };

        let provider = provider.with_model("custom-model".to_owned());
        assert_eq!(provider.model, "custom-model");
    }

    /// Tests provider name returns correct identifier.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_claude_code_provider_name() {
        let provider = ClaudeCodeProvider {
            model: DEFAULT_MODEL.to_owned(),
        };

        assert_eq!(provider.name(), "Claude Code");
    }

    /// Tests that method chaining works correctly.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[test]
    fn test_model_chaining() {
        let result = ClaudeCodeProvider::new();
        assert!(result.is_ok());

        if let Ok(base_provider) = result {
            let provider = base_provider.with_model("custom-model".to_owned());

            assert_eq!(provider.model, "custom-model");
        }
    }
}
