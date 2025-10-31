//! Embedding and vector search functionality.

mod bm25;
pub mod chunking;
mod client;
pub mod vector_search;

pub use bm25::BM25Index;
pub use chunking::{FileChunk, chunk_file};
#[cfg(any(test, feature = "test-helpers"))]
pub use client::FakeEmbeddingClient;
pub use client::{
    EmbeddingClient, EmbeddingProvider, SearchResult, VectorEntry, VectorStore, generate_preview,
};
pub use vector_search::{ProgressCallback, VectorSearchManager};
