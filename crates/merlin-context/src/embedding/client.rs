//! Embedding and vector search functionality using Ollama.

use crate::models::ModelConfig;
use merlin_core::{CoreResult as Result, Error};
use ollama_rs::Ollama;
use ollama_rs::generation::embeddings::request::GenerateEmbeddingsRequest;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::path::PathBuf;
use std::process::Command;

/// A single embedding vector
type Embedding = Vec<f32>;

/// Trait for generating embeddings from text
pub trait EmbeddingProvider: Send + Sync {
    /// Ensure the embedding model is available
    ///
    /// # Errors
    /// Returns an error if the model is not available or cannot be loaded
    fn ensure_model_available(&self) -> impl Future<Output = Result<()>> + Send;

    /// Generate embedding for text
    ///
    /// # Errors
    /// Returns an error if embedding generation fails
    fn embed(&self, text: &str) -> impl Future<Output = Result<Embedding>> + Send;

    /// Embed multiple texts in batch (sends all at once for better performance)
    ///
    /// # Errors
    /// Returns an error if any embedding generation fails
    fn embed_batch(
        &self,
        texts: Vec<String>,
    ) -> impl Future<Output = Result<Vec<Embedding>>> + Send;
}

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
#[derive(Default)]
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

        scores.sort_by(|first, second| second.1.partial_cmp(&first.1).unwrap_or(Ordering::Equal));

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
    pub fn len(&self) -> usize {
        self.embeddings.len()
    }

    /// Check if store is empty
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
pub struct OllamaEmbeddingClient {
    ollama: Ollama,
    model: String,
}

impl EmbeddingProvider for OllamaEmbeddingClient {
    async fn ensure_model_available(&self) -> Result<()> {
        // Check if Ollama is running by trying to list models
        let models = match self.ollama.list_local_models().await {
            Ok(models) => models,
            Err(error) => {
                return Err(Error::Other(format!(
                    "Failed to connect to Ollama: {error}.\n\nPlease ensure Ollama is installed and running:\n  - Install from: https://ollama.ai\n  - Start with: ollama serve"
                )));
            }
        };

        // Check if our embedding model is available
        let model_available = models.iter().any(|model| model.name.contains(&self.model));

        if !model_available {
            tracing::info!("⚙️  Embedding model '{}' not found", self.model);
            tracing::info!("⬇️  Pulling model from Ollama (this may take a few minutes)...");
            tracing::info!("    Running: ollama pull {}", self.model);

            // Pull the model using Ollama CLI with inherited stdio for progress
            let status = Command::new("ollama")
                .args(["pull", &self.model])
                .status()
                .map_err(|error| {
                    Error::Other(format!(
                        "Failed to run 'ollama pull {}': {}. Is Ollama installed?",
                        self.model, error
                    ))
                })?;

            if !status.success() {
                return Err(Error::Other(format!(
                    "Failed to pull model '{}'. Check Ollama is running.",
                    self.model
                )));
            }

            tracing::info!("✓ Successfully pulled embedding model '{}'", self.model);
        }

        Ok(())
    }

    async fn embed(&self, text: &str) -> Result<Embedding> {
        let request = GenerateEmbeddingsRequest::new(self.model.clone(), text.to_string().into());

        let response = self
            .ollama
            .generate_embeddings(request)
            .await
            .map_err(|error| {
                // Provide more detailed error message
                let error_str = format!("{error:?}");
                if error_str.contains("model") && error_str.contains("not found") {
                    Error::Other(format!(
                        "Embedding model '{}' not found. Run: ollama pull {}",
                        self.model, self.model
                    ))
                } else {
                    Error::Other(format!("Embedding generation failed: {error}"))
                }
            })?;

        // Ollama returns Vec<Vec<f32>>, we want the first embedding
        response
            .embeddings
            .into_iter()
            .next()
            .ok_or_else(|| Error::Other("No embeddings returned".into()))
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Embedding>> {
        if texts.is_empty() {
            return Ok(Vec::default());
        }

        // If single text, use regular embed
        if texts.len() == 1 {
            return Ok(vec![self.embed(&texts[0]).await?]);
        }

        // For multiple texts, send as batch
        let request = GenerateEmbeddingsRequest::new(self.model.clone(), texts.into());

        let response = self
            .ollama
            .generate_embeddings(request)
            .await
            .map_err(|error| {
                let error_str = format!("{error:?}");
                if error_str.contains("model") && error_str.contains("not found") {
                    Error::Other(format!(
                        "Embedding model '{}' not found. Run: ollama pull {}",
                        self.model, self.model
                    ))
                } else {
                    Error::Other(format!("Batch embedding generation failed: {error}"))
                }
            })?;

        Ok(response.embeddings)
    }
}

impl Default for OllamaEmbeddingClient {
    fn default() -> Self {
        let host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
        let config = ModelConfig::from_env();
        Self {
            ollama: Ollama::new(host, 11434),
            model: config.embedding,
        }
    }
}

/// Test-only fake embedding provider (deterministic, hash-based)
///
/// Available in test builds for fast, deterministic embeddings.
/// Use this for testing cache behavior, file operations, etc. without requiring Ollama.
#[cfg(test)]
pub struct FakeEmbeddingClient;

#[cfg(test)]
impl EmbeddingProvider for FakeEmbeddingClient {
    async fn ensure_model_available(&self) -> Result<()> {
        // No-op for fake embeddings
        Ok(())
    }

    async fn embed(&self, text: &str) -> Result<Embedding> {
        Ok(Self::fake_embedding(text))
    }

    async fn embed_batch(&self, texts: Vec<String>) -> Result<Vec<Embedding>> {
        Ok(texts
            .iter()
            .map(|text| Self::fake_embedding(text))
            .collect())
    }
}

#[cfg(test)]
impl FakeEmbeddingClient {
    /// Generate fake deterministic embedding for testing
    /// Uses simple hash of content to create a 384-dim vector
    fn fake_embedding(text: &str) -> Embedding {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash as _, Hasher as _};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        // Generate 384 dimensions (typical embedding size)
        // Use hash as seed for deterministic values
        let mut vec = Vec::with_capacity(384);
        for idx in 0..384 {
            let value = ((hash.wrapping_add(idx as u64)) % 1000) as f32 / 1000.0;
            vec.push(value);
        }
        vec
    }
}

/// Backward compatibility type alias
pub type EmbeddingClient = OllamaEmbeddingClient;

/// Calculate cosine similarity between two vectors
fn cosine_similarity(vector_a: &[f32], vector_b: &[f32]) -> f32 {
    if vector_a.len() != vector_b.len() {
        return 0.0;
    }

    let dot_product: f32 = vector_a
        .iter()
        .zip(vector_b.iter())
        .map(|(x, y)| x * y)
        .sum();
    let magnitude_a = vector_a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b = vector_b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

/// Generate a preview from file content (first few lines or summary)
pub fn generate_preview(content: &str, max_chars: usize) -> String {
    let lines: Vec<&str> = content.lines().take(10).collect();
    let preview = lines.join("\n");

    if preview.chars().count() > max_chars {
        // Use char boundaries to avoid panics with multi-byte characters
        let truncated: String = preview.chars().take(max_chars).collect();
        format!("{truncated}...")
    } else {
        preview
    }
}
