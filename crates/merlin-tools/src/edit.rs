use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::from_value;
use tokio::fs::{read_to_string, write};
use tracing::{debug, info};

use crate::tool::{Tool, ToolError, ToolInput, ToolOutput, ToolResult};

/// Parameters controlling how a file edit should be performed.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EditParams {
    /// Path to the file that will be modified.
    file_path: PathBuf,
    /// Text in the file that should be replaced.
    old_string: String,
    /// Replacement text that will be written to the file.
    new_string: String,
    #[serde(default)]
    /// When `true`, all occurrences of `old_string` are replaced. Otherwise exactly one replacement is made.
    replace_all: bool,
}

/// Tool that performs search-and-replace edits within files.
pub struct EditTool;

impl EditTool {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Apply the provided edit parameters to the target file.
    ///
    /// # Errors
    ///
    /// Returns a `ToolError` when the file does not exist, the replacement parameters are invalid,
    /// reading or writing the file fails, or when asynchronous I/O encounters an error.
    async fn perform_edit(&self, params: EditParams) -> ToolResult<ToolOutput> {
        debug!("Editing file: {:?}", params.file_path);

        if !params.file_path.exists() {
            return Err(ToolError::InvalidInput(format!(
                "File does not exist: {}",
                params.file_path.display()
            )));
        }

        let content = read_to_string(&params.file_path).await?;

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

        write(&params.file_path, new_content).await?;

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
        let params: EditParams = from_value(input.params)?;
        self.perform_edit(params).await
    }
}
