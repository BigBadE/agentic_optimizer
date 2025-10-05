use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::debug;

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ShowParams {
    file_path: PathBuf,
    #[serde(default)]
    start_line: Option<usize>,
    #[serde(default)]
    end_line: Option<usize>,
}

pub struct ShowTool;

impl ShowTool {
    #[must_use] 
    pub const fn new() -> Self {
        Self
    }

    async fn show_file(&self, params: ShowParams) -> ToolResult<ToolOutput> {
        debug!("Showing file: {:?}", params.file_path);

        if !params.file_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {}",
                params.file_path.display()
            )));
        }

        let content = fs::read_to_string(&params.file_path).await?;
        let lines: Vec<&str> = content.lines().collect();

        let start = params.start_line.unwrap_or(1).saturating_sub(1);
        let end = params.end_line.unwrap_or(lines.len()).min(lines.len());

        if start >= lines.len() {
            return Err(ToolError::InvalidInput(format!(
                "start_line {} exceeds file length {}",
                start + 1,
                lines.len()
            )));
        }

        let selected_lines: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(idx, line)| format!("{:5} | {}", start + idx + 1, line))
            .collect();

        let output = selected_lines.join("\n");

        Ok(ToolOutput::success_with_data(
            format!(
                "Showing lines {}-{} of {}",
                start + 1,
                end,
                params.file_path.display()
            ),
            serde_json::json!({ "content": output }),
        ))
    }
}

impl Default for ShowTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShowTool {
    fn name(&self) -> &'static str {
        "show"
    }

    fn description(&self) -> &'static str {
        "Show the contents of a file with line numbers. \
         Parameters: file_path (string), start_line (number, optional), \
         end_line (number, optional)"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let params: ShowParams = serde_json::from_value(input.params)?;
        self.show_file(params).await
    }
}
