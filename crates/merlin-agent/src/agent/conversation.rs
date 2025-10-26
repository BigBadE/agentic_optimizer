use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Maximum number of messages to keep in conversation history
const MAX_CONVERSATION_HISTORY: usize = 50;

/// Maximum number of recent files to track
const MAX_RECENT_FILES: usize = 20;
/// A single message in the conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Role of the message sender (user, assistant, system)
    pub role: String,
    /// Content of the message
    pub content: String,
    /// Unix timestamp when the message was created
    pub timestamp: u64,
    /// Files mentioned in this message
    pub files_mentioned: Vec<PathBuf>,
    /// Concepts discussed in this message
    pub concepts_discussed: Vec<String>,
}

/// Manages conversation state and context across multiple turns
#[derive(Debug, Clone, Default)]
pub struct ConversationManager {
    messages: VecDeque<ConversationMessage>,
    mentioned_files: VecDeque<PathBuf>,
    discussed_concepts: HashMap<String, usize>,
    current_focus: Option<PathBuf>,
    file_importance: HashMap<PathBuf, f32>,
}

impl ConversationManager {
    /// Create a new conversation manager
    pub fn new() -> Self {
        Self {
            messages: VecDeque::with_capacity(MAX_CONVERSATION_HISTORY),
            mentioned_files: VecDeque::with_capacity(MAX_RECENT_FILES),
            discussed_concepts: HashMap::new(),
            current_focus: None,
            file_importance: HashMap::new(),
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, role: String, content: String) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());

        let files_mentioned = Self::extract_file_mentions(&content);
        let concepts_discussed = Self::extract_concepts(&content);

        for file in &files_mentioned {
            self.track_file_mention(file.clone());
        }

        for concept in &concepts_discussed {
            *self.discussed_concepts.entry(concept.clone()).or_insert(0) += 1;
        }

        let message = ConversationMessage {
            role,
            content,
            timestamp,
            files_mentioned,
            concepts_discussed,
        };

        self.messages.push_back(message);

        if self.messages.len() > MAX_CONVERSATION_HISTORY
            && let Some(old_msg) = self.messages.pop_front()
        {
            self.decay_old_message(&old_msg);
        }
    }

    /// Track a file mention and update importance
    fn track_file_mention(&mut self, file: PathBuf) {
        if !self.mentioned_files.contains(&file) {
            self.mentioned_files.push_back(file.clone());
            if self.mentioned_files.len() > MAX_RECENT_FILES
                && let Some(old_file) = self.mentioned_files.pop_front()
            {
                self.file_importance.remove(&old_file);
            }
        }

        *self.file_importance.entry(file.clone()).or_insert(0.0) += 1.0;
        self.current_focus = Some(file);
    }

    /// Decay importance of concepts/files from old messages
    fn decay_old_message(&mut self, message: &ConversationMessage) {
        for concept in &message.concepts_discussed {
            if let Some(count) = self.discussed_concepts.get_mut(concept) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.discussed_concepts.remove(concept);
                }
            }
        }
    }

    /// Get conversation history as (role, content) pairs
    pub fn get_history(&self) -> Vec<(String, String)> {
        self.messages
            .iter()
            .map(|msg| (msg.role.clone(), msg.content.clone()))
            .collect()
    }

    /// Get recently mentioned files, sorted by importance
    pub fn get_recent_files(&self, limit: usize) -> Vec<PathBuf> {
        let mut files: Vec<_> = self.file_importance.iter().collect();
        files.sort_by(|file_a, file_b| file_b.1.partial_cmp(file_a.1).unwrap_or(Ordering::Equal));
        files
            .into_iter()
            .take(limit)
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// Get currently discussed concepts, sorted by frequency
    pub fn get_active_concepts(&self, limit: usize) -> Vec<String> {
        let mut concepts: Vec<_> = self.discussed_concepts.iter().collect();
        concepts.sort_by(|concept_a, concept_b| concept_b.1.cmp(concept_a.1));
        concepts
            .into_iter()
            .take(limit)
            .map(|(concept, _)| concept.clone())
            .collect()
    }

    /// Get the current focus file
    pub fn current_focus(&self) -> Option<&PathBuf> {
        self.current_focus.as_ref()
    }

    /// Set the current focus file explicitly
    pub fn set_focus(&mut self, file: PathBuf) {
        self.current_focus = Some(file.clone());
        self.track_file_mention(file);
    }

    /// Clear conversation history
    pub fn clear(&mut self) {
        self.messages.clear();
        self.mentioned_files.clear();
        self.discussed_concepts.clear();
        self.current_focus = None;
        self.file_importance.clear();
    }

    /// Get the number of messages in history
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Extract file mentions from text
    fn extract_file_mentions(text: &str) -> Vec<PathBuf> {
        let mut files = HashSet::new();

        let patterns = [
            r"([a-zA-Z0-9_\-./]+\.rs)",
            r"([a-zA-Z0-9_\-./]+\.toml)",
            r"([a-zA-Z0-9_\-./]+\.md)",
            r"([a-zA-Z0-9_\-./]+\.json)",
        ];

        for pattern in &patterns {
            let Ok(regex) = Regex::new(pattern) else {
                continue;
            };
            for cap in regex.captures_iter(text) {
                let Some(matched) = cap.get(1) else {
                    continue;
                };
                files.insert(PathBuf::from(matched.as_str()));
            }
        }

        files.into_iter().collect()
    }

    /// Extract concepts from text (simple keyword extraction)
    fn extract_concepts(text: &str) -> Vec<String> {
        let mut concepts = HashSet::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        for window in words.windows(2) {
            if window.len() == 2 {
                let bigram = format!("{} {}", window[0], window[1]);
                if Self::is_concept(&bigram) {
                    concepts.insert(bigram);
                }
            }
        }

        for word in words {
            if Self::is_concept(word) && word.len() > 4 {
                concepts.insert(word.to_string());
            }
        }

        concepts.into_iter().collect()
    }

    /// Check if a word/phrase is likely a concept
    fn is_concept(text: &str) -> bool {
        let lower = text.to_lowercase();

        let stopwords = [
            "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with",
            "by", "from", "up", "about", "into", "through", "during",
        ];

        !stopwords.contains(&lower.as_str())
            && !text.chars().all(char::is_numeric)
            && text.chars().any(char::is_alphanumeric)
    }

    /// Get a summary of the conversation state
    pub fn get_summary(&self) -> ConversationSummary {
        ConversationSummary {
            message_count: self.messages.len(),
            recent_files: self.get_recent_files(5),
            active_concepts: self.get_active_concepts(5),
            current_focus: self.current_focus.clone(),
        }
    }
}

/// Summary of conversation state
#[derive(Debug, Clone)]
pub struct ConversationSummary {
    /// Number of messages in the conversation
    pub message_count: usize,
    /// Recently mentioned files
    pub recent_files: Vec<PathBuf>,
    /// Active concepts being discussed
    pub active_concepts: Vec<String>,
    /// Current file focus
    pub current_focus: Option<PathBuf>,
}
#[cfg(test)]
mod tests {
    use super::*;

    // REMOVED: test_conversation_manager_creation - Constructor test only

    #[test]
    fn test_add_message() {
        let mut manager = ConversationManager::new();
        manager.add_message("user".to_owned(), "Hello".to_owned());
        assert_eq!(manager.message_count(), 1);
    }

    #[test]
    fn test_file_tracking() {
        let mut manager = ConversationManager::new();
        manager.add_message(
            "user".to_owned(),
            "Look at src/main.rs and src/lib.rs".to_owned(),
        );

        let recent = manager.get_recent_files(10);
        assert!(!recent.is_empty());
    }

    #[test]
    fn test_conversation_limit() {
        let mut manager = ConversationManager::new();

        for msg_num in 0..60 {
            manager.add_message("user".to_owned(), format!("Message {msg_num}"));
        }

        assert_eq!(manager.message_count(), MAX_CONVERSATION_HISTORY);
    }

    #[test]
    fn test_focus_tracking() {
        let mut manager = ConversationManager::new();
        let file = PathBuf::from("test.rs");

        manager.set_focus(file.clone());
        assert_eq!(manager.current_focus(), Some(&file));
    }

    #[test]
    fn test_clear() {
        let mut manager = ConversationManager::new();
        manager.add_message("user".to_owned(), "Test".to_owned());
        manager.clear();

        assert_eq!(manager.message_count(), 0);
        assert!(manager.current_focus().is_none());
    }
}
