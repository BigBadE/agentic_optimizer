use merlin_tools::{BashTool, EditTool, ShowTool, Tool, ToolInput};
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let show_tool = ShowTool::new();
    let edit_tool = EditTool::new();
    let bash_tool = BashTool::new();

    println!("Available tools:");
    println!("- {}: {}", show_tool.name(), show_tool.description());
    println!("- {}: {}", edit_tool.name(), edit_tool.description());
    println!("- {}: {}", bash_tool.name(), bash_tool.description());

    let bash_input = ToolInput {
        params: json!({
            "command": "echo Hello from tools!",
            "timeout_secs": 5
        }),
    };

    println!("\nExecuting bash tool...");
    let result = bash_tool.execute(bash_input).await?;
    println!("Result: {:?}", result);

    Ok(())
}

