use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{debug, info};

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EditParams {
    file_path: PathBuf,
    old_string: String,
    new_string: String,
    #[serde(default)]
    replace_all: bool,
}

pub struct EditTool;

impl EditTool {
    pub const fn new() -> Self {
        Self
    }

    async fn perform_edit(&self, params: EditParams) -> ToolResult<ToolOutput> {
        debug!("Editing file: {:?}", params.file_path);

        if !params.file_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {}",
                params.file_path.display()
            )));
        }

        let content = fs::read_to_string(&params.file_path).await?;

        if params.old_string == params.new_string {
            return Err(ToolError::InvalidInput(
                "old_string and new_string are identical".to_owned(),
            ));
        }

        let new_content = if params.replace_all {
            content.replace(&params.old_string, &params.new_string)
        } else {
            let occurrences = content.matches(&params.old_string).count();
            if occurrences == 0 {
                return Err(ToolError::InvalidInput(format!(
                    "old_string not found in file: {}",
                    params.file_path.display()
                )));
            }
            if occurrences > 1 {
                return Err(ToolError::InvalidInput(format!(
                    "old_string appears {occurrences} times in file, use replace_all=true or make it unique"
                )));
            }
            content.replacen(&params.old_string, &params.new_string, 1)
        };

        fs::write(&params.file_path, new_content).await?;

        info!("Successfully edited file: {}", params.file_path.display());
        Ok(ToolOutput::success(format!(
            "File edited successfully: {}",
            params.file_path.display()
        )))
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Edit a file by replacing old_string with new_string. \
         Parameters: file_path (string), old_string (string), new_string (string), \
         replace_all (bool, optional, default: false)"
    }

    async fn execute(&self, input: ToolInput) -> ToolResult<ToolOutput> {
        let params: EditParams = serde_json::from_value(input.params)?;
        self.perform_edit(params).await
    }
}
