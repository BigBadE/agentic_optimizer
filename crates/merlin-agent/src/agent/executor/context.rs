//! Context building for task execution

use merlin_context::ContextFetcher;
use merlin_core::{
    Context, Query, Result, RoutingError, Task,
    ui::{TaskProgress, UiChannel, UiEvent},
};
use std::fmt::Write as _;
use std::mem::replace;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::query_intent::QueryIntent;

/// Type alias for conversation history
pub type ConversationHistory = Vec<(String, String)>;

/// Context builder for agent execution
#[derive(Clone)]
pub struct ContextBuilder {
    context_fetcher: Arc<Mutex<ContextFetcher>>,
    /// Conversation history for context building
    pub conversation_history: Arc<Mutex<ConversationHistory>>,
}

impl ContextBuilder {
    /// Create new context builder
    #[must_use]
    pub fn new(
        context_fetcher: Arc<Mutex<ContextFetcher>>,
        conversation_history: Arc<Mutex<ConversationHistory>>,
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
        let intent = QueryIntent::classify(&task.description);
        let query = Query::new(task.description.clone());
        let task_id = task.id;

        // For conversational queries, return empty context (prompt added later)
        if intent == QueryIntent::Conversational {
            return Ok(Context::new(String::new()));
        }

        // For code queries, fetch file context as normal
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

        let mut fetcher = self.context_fetcher.lock().await;
        let project_root = fetcher.project_root().clone();
        *fetcher = replace(&mut *fetcher, ContextFetcher::new(project_root))
            .with_progress_callback(progress_callback);

        // Send substep for file gathering
        ui_channel.send(UiEvent::TaskStepStarted {
            task_id,
            step_id: "file_gathering".to_owned(),
            step_type: "thinking".to_owned(),
            content: "Searching for relevant files".to_owned(),
        });

        // Check if we have conversation history
        let context = {
            let conv_history = self.conversation_history.lock().await;
            if conv_history.is_empty() {
                drop(conv_history);
                fetcher
                    .build_context_for_query(&query)
                    .await
                    .map_err(|err| RoutingError::Other(format!("Failed to build context: {err}")))?
            } else {
                fetcher
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
    /// Returns an error if context building or prompt loading fails
    pub async fn build_context_for_typescript(
        &self,
        task: &Task,
        ui_channel: &UiChannel,
        tool_signatures: &str,
    ) -> Result<Context> {
        use merlin_core::prompts::load_prompt;

        // Load TypeScript agent prompt template
        let prompt_template = load_prompt("typescript_agent").map_err(|err| {
            RoutingError::Other(format!("Failed to load typescript_agent prompt: {err}"))
        })?;

        // Replace placeholder with actual signatures
        let system_prompt = prompt_template.replace("{TOOL_SIGNATURES}", tool_signatures);

        // Build base context (may include file context if relevant)
        let intent = QueryIntent::classify(&task.description);

        let mut context = if intent == QueryIntent::Conversational {
            Context::new(system_prompt)
        } else {
            // Get file context if needed
            let base_context = self.build_context(task, ui_channel).await?;

            // Combine TypeScript prompt with file context
            let mut combined = Context::new(system_prompt);
            combined.files = base_context.files;
            combined
        };

        // Add conversation history if present
        let conv_history = self.conversation_history.lock().await;
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
            let conv_history = self.conversation_history.lock().await;
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
            let conv_history = self.conversation_history.lock().await;
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

        let conv_history = self.conversation_history.lock().await;
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
