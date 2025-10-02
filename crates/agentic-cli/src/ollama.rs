//! Ollama installation and management utilities.

use std::process::{Command, Stdio};
use std::time::Duration;
use anyhow::{Result, Context as _};
use console::{style, Term};
use dialoguer::Confirm;

/// Check if Ollama is installed on the system
pub fn is_installed() -> bool {
    Command::new("ollama")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Check if Ollama is running and accessible
pub async fn is_running() -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap_or_default();
    
    client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map(|resp| resp.status().is_success())
        .unwrap_or(false)
}

/// Attempt to start Ollama server in the background
pub fn start_server() -> Result<()> {
    let term = Term::stdout();
    
    term.write_line(&format!("{}", style("Starting Ollama server...").cyan()))?;
    
    #[cfg(target_os = "windows")]
    {
        // On Windows, start Ollama in the background
        Command::new("cmd")
            .args(["/C", "start", "/B", "ollama", "serve"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to start Ollama server")?;
    }
    
    #[cfg(not(target_os = "windows"))]
    {
        // On Unix-like systems, start Ollama in the background
        Command::new("sh")
            .args(["-c", "ollama serve > /dev/null 2>&1 &"])
            .spawn()
            .context("Failed to start Ollama server")?;
    }
    
    term.write_line(&format!("{}", style("✓ Ollama server started").green()))?;
    term.write_line(&format!("{}", style("  Waiting for server to be ready...").dim()))?;
    
    Ok(())
}

/// Wait for Ollama to become available
pub async fn wait_for_ready(max_attempts: u32) -> Result<()> {
    for attempt in 1..=max_attempts {
        if is_running().await {
            return Ok(());
        }
        
        if attempt < max_attempts {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
    
    anyhow::bail!("Ollama server did not become ready after {} seconds", max_attempts)
}

/// Show Ollama not installed message and exit
pub fn show_not_installed_message() -> Result<()> {
    let term = Term::stdout();
    
    term.write_line("")?;
    term.write_line(&format!("{}", style("❌ Ollama is not installed!").red().bold()))?;
    term.write_line("")?;
    term.write_line("Ollama is required for intelligent context fetching.")?;
    term.write_line("It provides a local LLM that helps find relevant files for your queries.")?;
    term.write_line("")?;
    term.write_line(&format!("{}", style("Please install Ollama:").cyan().bold()))?;
    term.write_line("")?;
    
    #[cfg(target_os = "windows")]
    term.write_line("  Download from: https://ollama.ai/download/windows")?;
    
    #[cfg(target_os = "macos")]
    term.write_line("  Run: curl -fsSL https://ollama.ai/install.sh | sh")?;
    
    #[cfg(target_os = "linux")]
    term.write_line("  Run: curl -fsSL https://ollama.ai/install.sh | sh")?;
    
    term.write_line("")?;
    term.write_line("After installing, run this command again.")?;
    term.write_line("")?;
    
    anyhow::bail!("Ollama is not installed")
}

/// Ensure Ollama is installed and running, fails if not available
pub async fn ensure_available() -> Result<()> {
    let term = Term::stdout();
    
    // Check if installed
    if !is_installed() {
        show_not_installed_message()?;
    }
    
    // Check if running
    if !is_running().await {
        term.write_line("")?;
        term.write_line(&format!("{}", style("Ollama is not running").yellow()))?;
        
        let should_start = Confirm::new()
            .with_prompt("Would you like to start Ollama now?")
            .default(true)
            .interact()?;
        
        if !should_start {
            term.write_line("")?;
            term.write_line(&format!("{}", style("❌ Ollama is required").red().bold()))?;
            term.write_line("   You can start it manually with: ollama serve")?;
            term.write_line("")?;
            anyhow::bail!("Ollama server is not running");
        }
        
        start_server()?;
        
        // Wait for server to be ready
        wait_for_ready(10).await.context("Failed to start Ollama server")?;
        
        term.write_line(&format!("{}", style("✓ Ollama is ready!").green().bold()))?;
        term.write_line("")?;
    }
    
    // Check if the required model is available
    ensure_model_available().await?;
    
    Ok(())
}

/// Ensure the required model is downloaded
async fn ensure_model_available() -> Result<()> {
    let term = Term::stdout();
    let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "qwen2.5-coder:7b".to_string());
    
    term.write_line(&format!("{}", style(format!("Checking for model: {}", model)).dim()))?;
    
    // Check if model exists
    let output = Command::new("ollama")
        .args(["list"])
        .output()
        .context("Failed to list Ollama models")?;
    
    let models_list = String::from_utf8_lossy(&output.stdout);
    
    if !models_list.contains(&model) {
        term.write_line("")?;
        term.write_line(&format!("{}", style(format!("Model '{}' not found", model)).yellow()))?;
        
        let should_download = Confirm::new()
            .with_prompt(format!("Would you like to download {} now? (This may take a few minutes)", model))
            .default(true)
            .interact()?;
        
        if !should_download {
            term.write_line("")?;
            term.write_line(&format!("{}", style("❌ Model is required").red().bold()))?;
            term.write_line(&format!("   You can download it later with: ollama pull {}", model))?;
            term.write_line("")?;
            anyhow::bail!("Required model '{}' is not available", model);
        }
        
        term.write_line("")?;
        term.write_line(&format!("{}", style(format!("Downloading {}...", model)).cyan()))?;
        term.write_line(&format!("{}", style("This may take a few minutes depending on your connection.").dim()))?;
        term.write_line("")?;
        
        let status = Command::new("ollama")
            .args(["pull", &model])
            .status()
            .context("Failed to pull Ollama model")?;
        
        if !status.success() {
            anyhow::bail!("Failed to download model");
        }
        
        term.write_line("")?;
        term.write_line(&format!("{}", style("✓ Model downloaded successfully!").green().bold()))?;
        term.write_line("")?;
    } else {
        term.write_line(&format!("{}", style("✓ Model is available").green()))?;
    }
    
    Ok(())
}
