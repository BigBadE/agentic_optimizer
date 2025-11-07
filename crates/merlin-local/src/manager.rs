use reqwest::Client;
use std::time::Duration;

/// Manages Ollama installation and models
pub struct OllamaManager {
    /// HTTP client used to interact with the Ollama service.
    client: Client,
    /// Base URL pointing to the Ollama runtime.
    base_url: String,
}

impl OllamaManager {
    /// Sets a custom URL for the Ollama service.
    #[must_use]
    pub fn with_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Check if Ollama is running
    pub async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .timeout(Duration::from_millis(200))
            .send()
            .await
            .is_ok()
    }
}

impl Default for OllamaManager {
    fn default() -> Self {
        Self {
            client: Client::default(),
            base_url: "http://localhost:11434".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests Ollama manager creation with default URL.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn ollama_manager_creation() {
        let manager = OllamaManager::default();
        assert_eq!(manager.base_url, "http://localhost:11434");
    }

    /// Tests Ollama manager with custom URL configuration.
    ///
    /// # Panics
    /// Panics if assertions fail during test execution.
    #[tokio::test]
    async fn custom_url() {
        let manager = OllamaManager::default().with_url("http://custom:8080".to_owned());
        assert_eq!(manager.base_url, "http://custom:8080");
    }
}
