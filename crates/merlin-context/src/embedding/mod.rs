//! Embedding and vector search functionality.

mod bm25;
pub mod chunking;
mod client;
mod vector_search;

pub use bm25::BM25Index;
pub use chunking::{FileChunk, chunk_file};
pub use client::{EmbeddingClient, SearchResult, VectorEntry, VectorStore, generate_preview};
pub use vector_search::{ProgressCallback, VectorSearchManager};
