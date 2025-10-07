use anyhow::Result;
use merlin_tools::{BashTool, EditTool, ShowTool, Tool as _, ToolInput};
use serde_json::{json, to_string};
use std::io::stderr;
use tracing::info;
use tracing::subscriber::set_global_default;
use tracing_subscriber::fmt;

#[tokio::main]
async fn main() -> Result<()> {
    let subscriber = fmt().with_writer(stderr).finish();
    set_global_default(subscriber)?;

    let show_tool = ShowTool::new();
    let edit_tool = EditTool::new();
    let bash_tool = BashTool::new();

    info!("Available tools:");
    info!(tool = %show_tool.name(), description = %show_tool.description(), "Tool available");
    info!(tool = %edit_tool.name(), description = %edit_tool.description(), "Tool available");
    info!(tool = %bash_tool.name(), description = %bash_tool.description(), "Tool available");

    let bash_input = ToolInput {
        params: json!({
            "command": "echo Hello from tools!",
            "timeout_secs": 5
        }),
    };

    info!("Executing bash tool...");
    let result = bash_tool.execute(bash_input).await?;
    info!(result = %to_string(&result)?, "Execution completed");

    Ok(())
}
