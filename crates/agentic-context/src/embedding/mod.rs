//! Embedding and vector search functionality.

pub mod chunking;
mod client;
mod vector_search;
mod bm25;

pub use chunking::{FileChunk, chunk_file};
pub use client::{EmbeddingClient, VectorStore, VectorEntry, SearchResult, generate_preview};
pub use vector_search::VectorSearchManager;
pub use bm25::BM25Index;
