use async_trait::async_trait;
use reqwest::Client;
use std::time::Instant;
use agentic_core::{Context, ModelProvider, Query, Response, Result, TokenUsage};
use crate::models::{OllamaGenerateRequest, OllamaGenerateResponse};
use crate::OllamaManager;

/// Local model provider using Ollama
pub struct LocalModelProvider {
    client: Client,
    base_url: String,
    model_name: String,
    manager: OllamaManager,
}

impl LocalModelProvider {
    pub fn new(model_name: String) -> Self {
        Self {
            client: Client::new(),
            base_url: "http://localhost:11434".to_string(),
            model_name,
            manager: OllamaManager::new(),
        }
    }
    
    pub fn with_url(mut self, url: String) -> Self {
        self.base_url = url.clone();
        self.manager = self.manager.with_url(url);
        self
    }
    
    async fn generate_completion(&self, prompt: &str, system: Option<&str>) -> Result<OllamaGenerateResponse> {
        let request = OllamaGenerateRequest {
            model: self.model_name.clone(),
            prompt: prompt.to_string(),
            system: system.map(String::from),
            temperature: Some(0.7),
            max_tokens: None,
            stream: false,
        };
        
        let response = self.client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| agentic_core::Error::Other(format!("Ollama request failed: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(agentic_core::Error::Other(format!(
                "Ollama returned error: {}",
                response.status()
            )));
        }
        
        let ollama_response: OllamaGenerateResponse = response.json().await
            .map_err(|e| agentic_core::Error::Other(format!("Failed to parse Ollama response: {}", e)))?;
        
        Ok(ollama_response)
    }
}

#[async_trait]
impl ModelProvider for LocalModelProvider {
    fn name(&self) -> &'static str {
        "Ollama"
    }
    
    async fn is_available(&self) -> bool {
        self.manager.is_available().await
    }
    
    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let start = Instant::now();
        
        let system_prompt = "You are an expert coding assistant. Provide clear, concise, and correct code solutions.";
        
        let mut prompt = query.text.clone();
        
        if !context.files.is_empty() {
            prompt.push_str("\n\nContext files:\n");
            for file_ctx in &context.files {
                prompt.push_str(&format!("\n--- {} ---\n{}\n", 
                    file_ctx.path.display(), 
                    file_ctx.content
                ));
            }
        }
        
        let ollama_response = self.generate_completion(&prompt, Some(system_prompt)).await?;
        
        let latency_ms = start.elapsed().as_millis() as u64;
        
        let tokens_used = TokenUsage {
            input: ollama_response.prompt_eval_count as u64,
            output: ollama_response.eval_count as u64,
            cache_read: 0,
            cache_write: 0,
        };
        
        Ok(Response {
            text: ollama_response.response,
            confidence: 0.85,
            tokens_used,
            provider: format!("Ollama/{}", self.model_name),
            latency_ms,
        })
    }
    
    fn estimate_cost(&self, _context: &Context) -> f64 {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_provider_creation() {
        let provider = LocalModelProvider::new("qwen2.5-coder:7b".to_string());
        assert_eq!(provider.name(), "Ollama");
        assert_eq!(provider.model_name, "qwen2.5-coder:7b");
    }
    
    #[test]
    fn test_cost_estimation() {
        let provider = LocalModelProvider::new("qwen2.5-coder:7b".to_string());
        let context = Context::new("");
        assert_eq!(provider.estimate_cost(&context), 0.0);
    }
}
