use serde::{Deserialize, Serialize};

/// Local model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub name: String,
    pub size_bytes: u64,
    pub parameter_count: String,
    pub quantization: String,
    pub family: String,
}

impl ModelInfo {
    #[must_use]
    pub fn qwen_coder_7b() -> Self {
        Self {
            name: "qwen2.5-coder:7b".to_owned(),
            size_bytes: 4_400_000_000,
            parameter_count: "7B".to_owned(),
            quantization: "Q4_0".to_owned(),
            family: "qwen".to_owned(),
        }
    }

    #[must_use]
    pub fn deepseek_coder_6_7b() -> Self {
        Self {
            name: "deepseek-coder:6.7b".to_owned(),
            size_bytes: 3_800_000_000,
            parameter_count: "6.7B".to_owned(),
            quantization: "Q4_0".to_owned(),
            family: "deepseek".to_owned(),
        }
    }

    #[must_use]
    pub fn codellama_7b() -> Self {
        Self {
            name: "codellama:7b".to_owned(),
            size_bytes: 3_800_000_000,
            parameter_count: "7B".to_owned(),
            quantization: "Q4_0".to_owned(),
            family: "llama".to_owned(),
        }
    }
}

/// Ollama API response for model list
#[derive(Debug, Deserialize)]
pub struct OllamaListResponse {
    pub models: Vec<OllamaModel>,
}

#[derive(Debug, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: String,
}

/// Ollama API request for generation
#[derive(Debug, Serialize)]
pub struct OllamaGenerateRequest {
    pub model: String,
    pub prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    pub stream: bool,
}

/// Ollama API response for generation
#[derive(Debug, Deserialize)]
pub struct OllamaGenerateResponse {
    pub model: String,
    pub response: String,
    pub done: bool,
    #[serde(default)]
    pub total_duration: u64,
    #[serde(default)]
    pub prompt_eval_count: usize,
    #[serde(default)]
    pub eval_count: usize,
}
