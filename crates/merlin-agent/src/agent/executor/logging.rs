//! Context and execution logging utilities

use merlin_core::{Context, Task};

use super::context::ContextBuilder;

/// Logging utilities for context and execution
pub struct ContextLogger;

impl ContextLogger {
    /// Calculate files token count
    #[must_use]
    pub fn calculate_files_tokens(context: &Context) -> usize {
        let char_count: usize = context.files.iter().map(|f| f.content.len()).sum();
        char_count / 4
    }

    /// Log context breakdown to debug.log
    pub async fn log_context_breakdown(context: &Context, context_builder: &ContextBuilder) {
        use tracing::info;
        const BAR_WIDTH: usize = 50;

        info!("=====================================");
        info!("CONTEXT USAGE BREAKDOWN");
        info!("=====================================");

        // Calculate token counts
        let conversation_tokens = context_builder.calculate_conversation_tokens().await;
        let total_files_tokens = Self::calculate_files_tokens(context);
        let system_prompt_tokens = context.system_prompt.len() / 4;
        let total_tokens = context.token_estimate();

        info!("Total tokens: ~{}", total_tokens);
        info!("");

        // Display bar chart breakdown
        Self::log_token_bars(
            conversation_tokens,
            total_files_tokens,
            system_prompt_tokens,
            total_tokens,
            BAR_WIDTH,
        );

        info!("=====================================");

        // Conversation preview
        context_builder.log_conversation_preview().await;

        // File breakdown
        Self::log_file_breakdown(context);

        info!("=====================================");
    }

    /// Log token distribution bar charts
    fn log_token_bars(
        conversation_tokens: usize,
        files_tokens: usize,
        system_tokens: usize,
        total_tokens: usize,
        bar_width: usize,
    ) {
        use tracing::info;

        if total_tokens == 0 {
            return;
        }

        let conv_bar = if conversation_tokens > 0 {
            (conversation_tokens * bar_width / total_tokens).max(1)
        } else {
            0
        };
        let files_bar = if files_tokens > 0 {
            (files_tokens * bar_width / total_tokens).max(1)
        } else {
            0
        };
        let system_bar = if system_tokens > 0 {
            (system_tokens * bar_width / total_tokens).max(1)
        } else {
            0
        };

        info!(
            "Conversation:  {:>6} tokens ({:>5.1}%) {}",
            conversation_tokens,
            (conversation_tokens as f64 / total_tokens as f64) * 100.0,
            "█".repeat(conv_bar)
        );
        info!(
            "Files:         {:>6} tokens ({:>5.1}%) {}",
            files_tokens,
            (files_tokens as f64 / total_tokens as f64) * 100.0,
            "█".repeat(files_bar)
        );
        info!(
            "System Prompt: {:>6} tokens ({:>5.1}%) {}",
            system_tokens,
            (system_tokens as f64 / total_tokens as f64) * 100.0,
            "█".repeat(system_bar)
        );
    }

    /// Log file breakdown
    fn log_file_breakdown(_context: &Context) {
        // File list is now printed by the context builder
    }

    /// Dump full context to debug.log
    pub async fn dump_context_to_log(
        context: &Context,
        task: &Task,
        context_builder: &ContextBuilder,
    ) {
        use tracing::info;

        info!("================== CONTEXT DUMP ==================");
        info!("Task: {}", task.description);
        info!("");

        context_builder.log_conversation_history().await;
        Self::log_system_prompt(context);
        Self::log_statistics(context);

        info!("================================================");
    }

    /// Log system prompt section
    fn log_system_prompt(context: &Context) {
        use tracing::info;

        info!("=== SYSTEM PROMPT ===");
        info!("{}", context.system_prompt);
        info!("");
    }

    /// Log statistics section
    fn log_statistics(context: &Context) {
        use tracing::info;

        info!("=== STATISTICS ===");
        info!("Estimated tokens: {}", context.token_estimate());
        info!("Files: {}", context.files.len());
        info!(
            "System prompt length: {} chars",
            context.system_prompt.len()
        );
    }
}
