use reqwest::Client;
use crate::{LocalError, Result};
use crate::models::{ModelInfo, OllamaListResponse, OllamaModel};

/// Manages Ollama installation and models
pub struct OllamaManager {
    client: Client,
    base_url: String,
}

impl OllamaManager {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
        }
    }
    
    pub fn with_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }
    
    /// Check if Ollama is running
    pub async fn is_available(&self) -> bool {
        self.client
            .get(&format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }
    
    /// List installed models
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>> {
        let response = self.client
            .get(&format!("{}/api/tags", self.base_url))
            .send()
            .await
            .map_err(|e| LocalError::OllamaUnavailable(e.to_string()))?;
        
        let list: OllamaListResponse = response.json().await?;
        Ok(list.models)
    }
    
    /// Check if a specific model is installed
    pub async fn has_model(&self, model_name: &str) -> Result<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.name.starts_with(model_name)))
    }
    
    /// Pull a model from Ollama registry
    pub async fn pull_model(&self, model_name: &str) -> Result<()> {
        let response = self.client
            .post(&format!("{}/api/pull", self.base_url))
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
    pub async fn ensure_model(&self, model_name: &str) -> Result<()> {
        if !self.has_model(model_name).await? {
            self.pull_model(model_name).await?;
        }
        Ok(())
    }
    
    /// Get recommended models for code tasks
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

    #[tokio::test]
    async fn test_ollama_manager_creation() {
        let manager = OllamaManager::new();
        assert_eq!(manager.base_url, "http://localhost:11434");
    }
    
    #[tokio::test]
    async fn test_custom_url() {
        let manager = OllamaManager::new()
            .with_url("http://custom:8080".to_string());
        assert_eq!(manager.base_url, "http://custom:8080");
    }
    
    #[test]
    fn test_recommended_models() {
        let models = OllamaManager::recommended_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.name.contains("qwen")));
    }
}
