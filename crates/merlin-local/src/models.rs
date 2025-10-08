use serde::{Deserialize, Serialize};

/// Local model metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier used in Ollama.
    pub name: String,
    /// Approximate size in bytes.
    pub size_bytes: u64,
    /// Human-readable parameter count (e.g., "7B").
    pub parameter_count: String,
    /// Quantization format (e.g., "`Q4_0`").
    pub quantization: String,
    /// Model family (e.g., "qwen", "llama").
    pub family: String,
}

impl ModelInfo {
    /// Creates metadata for the Qwen 2.5 Coder 7B model.
    pub fn qwen_coder_7b() -> Self {
        Self {
            name: "qwen2.5-coder:7b".to_owned(),
            size_bytes: 4_400_000_000,
            parameter_count: "7B".to_owned(),
            quantization: "Q4_0".to_owned(),
            family: "qwen".to_owned(),
        }
    }

    /// Creates metadata for the `DeepSeek` Coder 6.7B model.
    pub fn deepseek_coder_6_7b() -> Self {
        Self {
            name: "deepseek-coder:6.7b".to_owned(),
            size_bytes: 3_800_000_000,
            parameter_count: "6.7B".to_owned(),
            quantization: "Q4_0".to_owned(),
            family: "deepseek".to_owned(),
        }
    }

    /// Creates metadata for the `CodeLlama` 7B model.
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
    /// List of models installed in Ollama.
    pub models: Vec<OllamaModel>,
}

/// Information about an Ollama model returned from the API.
#[derive(Debug, Deserialize)]
pub struct OllamaModel {
    /// Model identifier.
    pub name: String,
    /// Size of the model in bytes.
    pub size: u64,
    /// Content digest for the model.
    pub digest: String,
    /// Timestamp of last modification.
    pub modified_at: String,
}

/// Ollama API request for generation
#[derive(Debug, Serialize)]
pub struct OllamaGenerateRequest {
    /// Model to use for generation.
    pub model: String,
    /// Input prompt for the model.
    pub prompt: String,
    /// Optional system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Sampling temperature (0.0 to 1.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<usize>,
    /// Whether to stream the response.
    pub stream: bool,
}

/// Ollama API response for generation
#[derive(Debug, Deserialize)]
pub struct OllamaGenerateResponse {
    /// Model that generated the response.
    pub model: String,
    /// Generated text content.
    pub response: String,
    /// Whether generation is complete.
    pub done: bool,
    /// Total time taken in nanoseconds.
    #[serde(default)]
    pub total_duration: u64,
    /// Number of tokens in the prompt.
    #[serde(default)]
    pub prompt_eval_count: usize,
    /// Number of tokens generated.
    #[serde(default)]
    pub eval_count: usize,
}
