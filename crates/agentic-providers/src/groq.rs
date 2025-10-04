use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Instant;
use agentic_core::{Context, Error, ModelProvider, Query, Response, Result, TokenUsage};

const GROQ_API_URL: &str = "https://api.groq.com/openai/v1/chat/completions";
const DEFAULT_MODEL: &str = "llama-3.1-70b-versatile";

/// Groq API provider (free tier with rate limits)
pub struct GroqProvider {
    client: Client,
    api_key: String,
    model: String,
}

impl GroqProvider {
    pub fn new() -> Result<Self> {
        let api_key = env::var("GROQ_API_KEY")
            .map_err(|_| Error::Other("GROQ_API_KEY not set".to_string()))?;
        
        Ok(Self {
            client: Client::new(),
            api_key,
            model: DEFAULT_MODEL.to_string(),
        })
    }
    
    pub fn with_model(mut self, model: String) -> Self {
        self.model = model;
        self
    }
    
    pub fn with_api_key(mut self, api_key: String) -> Self {
        self.api_key = api_key;
        self
    }
}

#[derive(Debug, Serialize)]
struct GroqRequest {
    model: String,
    messages: Vec<GroqMessage>,
    temperature: f32,
    max_tokens: usize,
}

#[derive(Debug, Serialize)]
struct GroqMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
    usage: GroqUsage,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqResponseMessage,
}

#[derive(Debug, Deserialize)]
struct GroqResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct GroqUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
}

#[async_trait]
impl ModelProvider for GroqProvider {
    fn name(&self) -> &'static str {
        "Groq"
    }
    
    async fn is_available(&self) -> bool {
        !self.api_key.is_empty()
    }
    
    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let start = Instant::now();
        
        let mut messages = vec![
            GroqMessage {
                role: "system".to_string(),
                content: "You are an expert coding assistant. Provide clear, concise, and correct code solutions.".to_string(),
            }
        ];
        
        let mut user_content = query.text.clone();
        
        if !context.files.is_empty() {
            user_content.push_str("\n\nContext files:\n");
            for file_ctx in &context.files {
                user_content.push_str(&format!("\n--- {} ---\n{}\n", 
                    file_ctx.path.display(), 
                    file_ctx.content
                ));
            }
        }
        
        messages.push(GroqMessage {
            role: "user".to_string(),
            content: user_content,
        });
        
        let request = GroqRequest {
            model: self.model.clone(),
            messages,
            temperature: 0.7,
            max_tokens: 8000,
        };
        
        let response = self.client
            .post(GROQ_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::Other(format!("Groq API request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::Other(format!("Groq API error {}: {}", status, error_text)));
        }
        
        let groq_response: GroqResponse = response.json().await
            .map_err(|e| Error::Other(format!("Failed to parse Groq response: {}", e)))?;
        
        let latency_ms = start.elapsed().as_millis() as u64;
        
        let text = groq_response.choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| Error::Other("No response from Groq".to_string()))?;
        
        let tokens_used = TokenUsage {
            input: groq_response.usage.prompt_tokens as u64,
            output: groq_response.usage.completion_tokens as u64,
            cache_read: 0,
            cache_write: 0,
        };
        
        Ok(Response {
            text,
            confidence: 0.9,
            tokens_used,
            provider: format!("Groq/{}", self.model),
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
    fn test_groq_provider_with_api_key() {
        let provider = GroqProvider {
            client: Client::new(),
            api_key: "test_key".to_string(),
            model: DEFAULT_MODEL.to_string(),
        };
        
        assert_eq!(provider.name(), "Groq");
        assert_eq!(provider.model, DEFAULT_MODEL);
    }
    
    #[tokio::test]
    async fn test_groq_availability() {
        let provider = GroqProvider {
            client: Client::new(),
            api_key: "test_key".to_string(),
            model: DEFAULT_MODEL.to_string(),
        };
        
        assert!(provider.is_available().await);
    }
    
    #[test]
    fn test_cost_estimation() {
        let provider = GroqProvider {
            client: Client::new(),
            api_key: "test_key".to_string(),
            model: DEFAULT_MODEL.to_string(),
        };
        
        let context = Context::new("");
        assert_eq!(provider.estimate_cost(&context), 0.0);
    }
}
