//! Context building for task execution

use merlin_context::ContextFetcher;
use merlin_core::{
    Context, Query, Result, RoutingError, Task,
    ui::{TaskProgress, UiChannel, UiEvent},
};
use std::fmt::Write as _;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type alias for conversation history
pub type ConversationHistory = Vec<(String, String)>;

/// Context builder for agent execution
#[derive(Clone)]
pub struct ContextBuilder {
    context_fetcher: Arc<ContextFetcher>,
    /// Conversation history for context building (`RwLock` for read-heavy access)
    pub conversation_history: Arc<RwLock<ConversationHistory>>,
}

impl ContextBuilder {
    /// Create new context builder
    #[must_use]
    pub fn new(
        context_fetcher: Arc<ContextFetcher>,
        conversation_history: Arc<RwLock<ConversationHistory>>,
    ) -> Self {
        Self {
            context_fetcher,
            conversation_history,
        }
    }

    /// Build context for a task
    ///
    /// # Errors
    /// Returns an error if context building fails
    pub async fn build_context(&self, task: &Task, ui_channel: &UiChannel) -> Result<Context> {
        let query = Query::new(task.description.clone());
        let task_id = task.id;

        // Always fetch file context (self-assessor will handle simple tasks)
        let ui_clone = ui_channel.clone();

        let progress_callback = Arc::new(move |stage: &str, current: u64, total: Option<u64>| {
            ui_clone.send(UiEvent::TaskProgress {
                task_id,
                progress: TaskProgress {
                    stage: stage.to_owned(),
                    current,
                    total,
                    message: format!(
                        "{} ({}/{})",
                        stage,
                        current,
                        total.map_or_else(|| "?".to_owned(), |total_val| total_val.to_string())
                    ),
                },
            });
        });

        // Update progress callback without destroying cached state
        self.context_fetcher
            .set_progress_callback(progress_callback)
            .await;

        // Send substep for file gathering
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "file_gathering".to_owned(),
            step_type: "thinking".to_owned(),
            content: "Searching for relevant files".to_owned(),
        });

        // Check if we have conversation history (read lock)
        let context = {
            let conv_history = self.conversation_history.read().await;
            if conv_history.is_empty() {
                drop(conv_history);
                self.context_fetcher
                    .build_context_for_query(&query)
                    .await
                    .map_err(|err| RoutingError::Other(format!("Failed to build context: {err}")))?
            } else {
                self.context_fetcher
                    .build_context_from_conversation(&conv_history, &query)
                    .await
                    .map_err(|err| RoutingError::Other(format!("Failed to build context: {err}")))?
            }
        };

        ui_channel.send(UiEvent::TaskStepCompleted {
            task_id,
            step_id: "file_gathering".to_owned(),
        });

        Ok(context)
    }

    /// Build context for TypeScript-based agent execution
    ///
    /// # Errors
    /// Returns an error if context building fails
    pub async fn build_context_for_typescript(
        &self,
        task: &Task,
        ui_channel: &UiChannel,
        compiled_prompt: &str,
    ) -> Result<Context> {
        // Always get file context (self-assessor handles simple tasks before we get here)
        let base_context = self.build_context(task, ui_channel).await?;

        // Combine pre-compiled TypeScript prompt with file context
        let mut context = Context::new(compiled_prompt.to_owned());
        context.files = base_context.files;

        // Add conversation history if present (read lock)
        let conv_history = self.conversation_history.read().await;
        if !conv_history.is_empty() {
            let _write_result1 = write!(context.system_prompt, "\n\n## Conversation History\n\n");

            for (role, content) in conv_history.iter() {
                let _write_result2 = writeln!(context.system_prompt, "{role}: {content}");
            }
        }

        Ok(context)
    }

    /// Calculate conversation token count
    #[must_use]
    pub async fn calculate_conversation_tokens(&self) -> usize {
        let char_count: usize = {
            let conv_history = self.conversation_history.read().await;
            conv_history
                .iter()
                .map(|(role, content)| role.len() + content.len() + 10)
                .sum()
        };
        char_count / 4
    }

    /// Log conversation preview
    pub async fn log_conversation_preview(&self) {
        use tracing::info;

        let (message_count, messages, has_more) = {
            let conv_history = self.conversation_history.read().await;
            if conv_history.is_empty() {
                return;
            }

            let preview_count = conv_history.len().min(3);
            let messages: Vec<_> = conv_history
                .iter()
                .rev()
                .take(preview_count)
                .enumerate()
                .map(|(idx, (role, content))| {
                    let preview = if content.len() > 60 {
                        format!("{}...", &content[..60])
                    } else {
                        content.clone()
                    };
                    (idx, role.clone(), preview)
                })
                .collect();

            let has_more = conv_history.len() > preview_count;
            let more_count = if has_more {
                conv_history.len() - preview_count
            } else {
                0
            };

            (conv_history.len(), messages, (has_more, more_count))
        };

        info!("💬 Conversation: {} messages", message_count);
        for (idx, role, preview) in messages {
            info!("  [{idx}] {role}: {preview}");
        }
        if has_more.0 {
            info!("  ... and {} more", has_more.1);
        }
        info!("");
    }

    /// Log conversation history section
    pub async fn log_conversation_history(&self) {
        use tracing::info;

        let conv_history = self.conversation_history.read().await;
        if !conv_history.is_empty() {
            info!(
                "=== CONVERSATION HISTORY ({} messages) ===",
                conv_history.len()
            );
            for (idx, (role, content)) in conv_history.iter().enumerate() {
                info!("[{idx}] {role}:");
                info!("{content}");
                info!("");
            }
        }
    }
}
