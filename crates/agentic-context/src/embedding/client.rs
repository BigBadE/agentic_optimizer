//! Embedding and vector search functionality using Ollama.

use std::path::PathBuf;
use std::collections::HashMap;
use ollama_rs::Ollama;
use agentic_core::Result;
use crate::models::ModelConfig;

/// A single embedding vector
type Embedding = Vec<f32>;

/// Vector search result
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// File path
    pub file_path: PathBuf,
    /// Similarity score (0.0 to 1.0)
    pub score: f32,
    /// Preview of file content
    pub preview: String,
    /// BM25 keyword score (if available)
    pub bm25_score: Option<f32>,
    /// Vector semantic score (if available)
    pub vector_score: Option<f32>,
}

/// In-memory vector database for code files
pub struct VectorStore {
    /// File path to embedding mapping
    embeddings: HashMap<PathBuf, Embedding>,
    /// File path to content preview
    previews: HashMap<PathBuf, String>,
}

/// Entry in the vector store for iteration
#[derive(Debug, Clone)]
pub struct VectorEntry {
    /// File path
    pub path: PathBuf,
    /// Embedding vector
    pub embedding: Vec<f32>,
    /// Content preview
    pub preview: String,
}

impl VectorStore {
    /// Create a new empty vector store
    #[must_use]
    pub fn new() -> Self {
        Self {
            embeddings: HashMap::new(),
            previews: HashMap::new(),
        }
    }

    /// Add a file embedding to the store
    pub fn add(&mut self, path: PathBuf, embedding: Embedding, preview: String) {
        self.embeddings.insert(path.clone(), embedding);
        self.previews.insert(path, preview);
    }

    /// Search for similar files
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<SearchResult> {
        let mut scores: Vec<(PathBuf, f32)> = self
            .embeddings
            .iter()
            .map(|(path, emb)| {
                let score = cosine_similarity(query_embedding, emb);
                (path.clone(), score)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores
            .into_iter()
            .take(top_k)
            .filter_map(|(path, score)| {
                self.previews.get(&path).map(|preview| SearchResult {
                    file_path: path,
                    score,
                    preview: preview.clone(),
                    bm25_score: None,
                    vector_score: None,
                })
            })
            .collect()
    }

    /// Get number of stored embeddings
    #[must_use]
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    /// Check if store is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.embeddings.is_empty()
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = VectorEntry> + '_ {
        self.embeddings.iter().filter_map(|(path, embedding)| {
            self.previews.get(path).map(|preview| VectorEntry {
                path: path.clone(),
                embedding: embedding.clone(),
                preview: preview.clone(),
            })
        })
    }
}

/// Ollama embedding client
pub struct EmbeddingClient {
    ollama: Ollama,
    model: String,
}

impl EmbeddingClient {
    /// Create a new embedding client
    #[must_use]
    pub fn new() -> Self {
        let host = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        
        let config = ModelConfig::from_env();
        
        Self {
            ollama: Ollama::new(host, 11434),
            model: config.embedding,
        }
    }

    /// Ensure the embedding model is available
    pub async fn ensure_model_available(&self) -> Result<()> {
        use std::process::Command;
        
        // List local models
        let models = self.ollama.list_local_models()
            .await
            .map_err(|e| agentic_core::Error::Other(format!("Failed to list Ollama models: {e}")))?;

        // Check if our embedding model is available
        let model_available = models.iter().any(|m| m.name.contains(&self.model));

        if !model_available {
            eprintln!("⚙️  Embedding model '{}' not found", self.model);
            eprintln!("⬇️  Pulling model from Ollama (this may take a few minutes)...");
            eprintln!("    Running: ollama pull {}", self.model);
            eprintln!();
            
            // Pull the model using Ollama CLI with inherited stdio for progress
            let status = Command::new("ollama")
                .args(["pull", &self.model])
                .status()
                .map_err(|e| agentic_core::Error::Other(
                    format!("Failed to run 'ollama pull {}': {}. Is Ollama installed?", self.model, e)
                ))?;

            if !status.success() {
                return Err(agentic_core::Error::Other(
                    format!("Failed to pull model '{}'. Check Ollama is running.", self.model)
                ));
            }

            eprintln!();
            eprintln!("✓ Successfully pulled embedding model '{}'", self.model);
        }

        Ok(())
    }

    /// Generate embedding for text
    pub async fn embed(&self, text: &str) -> Result<Embedding> {
        use ollama_rs::generation::embeddings::request::GenerateEmbeddingsRequest;

        let request = GenerateEmbeddingsRequest::new(self.model.clone(), text.to_string().into());

        let response = self
            .ollama
            .generate_embeddings(request)
            .await
            .map_err(|e| {
                // Provide more detailed error message
                let error_str = format!("{e:?}");
                if error_str.contains("model") && error_str.contains("not found") {
                    agentic_core::Error::Other(
                        format!("Embedding model '{}' not found. Run: ollama pull {}", self.model, self.model)
                    )
                } else {
                    agentic_core::Error::Other(format!("Embedding generation failed: {e}"))
                }
            })?;

        // Ollama returns Vec<Vec<f32>>, we want the first embedding
        response.embeddings.into_iter().next()
            .ok_or_else(|| agentic_core::Error::Other("No embeddings returned".into()))
    }

    /// Embed multiple texts in batch
    pub async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Embedding>> {
        let mut embeddings = Vec::new();
        
        for text in texts {
            embeddings.push(self.embed(&text).await?);
        }
        
        Ok(embeddings)
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Generate a preview from file content (first few lines or summary)
#[allow(dead_code)]
pub fn generate_preview(content: &str, max_chars: usize) -> String {
    let lines: Vec<&str> = content.lines().take(10).collect();
    let preview = lines.join("\n");
    
    if preview.chars().count() > max_chars {
        // Use char boundaries to avoid panics with multi-byte characters
        let truncated: String = preview.chars().take(max_chars).collect();
        format!("{}...", truncated)
    } else {
        preview
    }
}

impl Default for VectorStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EmbeddingClient {
    fn default() -> Self {
        Self::new()
    }
}
