use crate::models::{ModelInfo, OllamaListResponse, OllamaModel};
use crate::{LocalError, Result};
use reqwest::Client;

/// Manages Ollama installation and models
pub struct OllamaManager {
    /// HTTP client used to interact with the Ollama service.
    client: Client,
    /// Base URL pointing to the Ollama runtime.
    base_url: String,
}

impl OllamaManager {
    #[must_use]
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_owned(),
        }
    }

    #[must_use]
    pub fn with_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Check if Ollama is running
    pub async fn is_available(&self) -> bool {
        self.client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }

    /// List installed models
    ///
    /// # Errors
    ///
    /// Returns an error if Ollama is not available or if the response cannot be parsed
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>> {
        let response = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|err| LocalError::OllamaUnavailable(err.to_string()))?;

        let list: OllamaListResponse = response.json().await?;
        Ok(list.models)
    }

    /// Check if a specific model is installed
    ///
    /// # Errors
    ///
    /// Returns an error if the model list cannot be retrieved
    pub async fn has_model(&self, model_name: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models
            .iter()
            .any(|model| model.name.starts_with(model_name)))
    }

    /// Pull a model from Ollama registry
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be pulled
    pub async fn pull_model(&self, model_name: &str) -> Result<()> {
        let response = self
            .client
            .post(format!("{}/api/pull", self.base_url))
            .json(&serde_json::json!({
                "name": model_name,
                "stream": false
            }))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(LocalError::ModelPullFailed(format!(
                "Failed to pull model {}: {}",
                model_name,
                response.status()
            )))
        }
    }

    /// Ensure a model is available, pulling if necessary
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot be verified or pulled
    pub async fn ensure_model(&self, model_name: &str) -> Result<()> {
        if !self.has_model(model_name).await? {
            self.pull_model(model_name).await?;
        }
        Ok(())
    }

    /// Get recommended models for code tasks
    #[must_use]
    pub fn recommended_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo::qwen_coder_7b(),
            ModelInfo::deepseek_coder_6_7b(),
            ModelInfo::codellama_7b(),
        ]
    }
}

impl Default for OllamaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// # Panics
    /// Panics if manager base URL doesn't match expected default.
    #[tokio::test]
    async fn ollama_manager_creation() {
        let manager = OllamaManager::new();
        assert_eq!(manager.base_url, "http://localhost:11434");
    }

    /// # Panics
    /// Panics if manager base URL doesn't match custom URL.
    #[tokio::test]
    async fn custom_url() {
        let manager = OllamaManager::new().with_url("http://custom:8080".to_owned());
        assert_eq!(manager.base_url, "http://custom:8080");
    }

    /// # Panics
    /// Panics if recommended models list is empty or doesn't contain expected models.
    #[test]
    fn recommended_models() {
        let models = OllamaManager::recommended_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|model| model.name.contains("qwen")));
    }
}
