//! Vector search manager with persistent caching.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use serde::{Deserialize, Serialize};
use indicatif::{ProgressBar, ProgressStyle};
use tokio::task::JoinSet;

use agentic_core::Result;
use crate::embedding::{EmbeddingClient, VectorStore, SearchResult, generate_preview, BM25Index};
use crate::embedding::chunking::chunk_file;
use crate::fs_utils::is_source_file;
use crate::context_inclusion::MIN_SIMILARITY_SCORE;

/// Cache entry for a chunk embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedEmbedding {
    /// File path
    path: PathBuf,
    /// Chunk identifier
    chunk_id: String,
    /// Start line
    start_line: usize,
    /// End line
    end_line: usize,
    /// Embedding vector
    embedding: Vec<f32>,
    /// Chunk content preview
    preview: String,
    /// Last modification time
    modified: SystemTime,
}

/// Cached vector database
#[derive(Debug, Serialize, Deserialize)]
struct VectorCache {
    /// Version identifier for cache invalidation
    version: u32,
    /// Cached embeddings
    embeddings: Vec<CachedEmbedding>,
}

impl VectorCache {
    const VERSION: u32 = 2;  // Bumped for chunk-based embeddings

    fn new() -> Self {
        Self {
            version: Self::VERSION,
            embeddings: Vec::new(),
        }
    }

    fn is_valid(&self) -> bool {
        self.version == Self::VERSION
    }
}

/// Vector search manager with caching and BM25 keyword search
pub struct VectorSearchManager {
    /// In-memory vector store
    store: VectorStore,
    /// BM25 keyword search index
    bm25: BM25Index,
    /// File modification times for cache invalidation
    file_times: HashMap<PathBuf, SystemTime>,
    /// Embedding client
    client: EmbeddingClient,
    /// Project root
    project_root: PathBuf,
    /// Cache file path
    cache_path: PathBuf,
}

impl VectorSearchManager {
    /// Create a new vector search manager
    pub fn new(project_root: PathBuf) -> Self {
        let cache_path = project_root.join(".agentic_cache").join("embeddings.bin");
        
        Self {
            store: VectorStore::new(),
            bm25: BM25Index::new(),
            file_times: HashMap::new(),
            client: EmbeddingClient::new(),
            project_root,
            cache_path,
        }
    }

    /// Initialize vector store by loading from cache or generating embeddings
    pub async fn initialize(&mut self) -> Result<()> {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.cyan} {msg}")
                .unwrap_or_else(|_| ProgressStyle::default_spinner())
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        // Check if embedding model is available
        spinner.set_message("Checking embedding model availability...");
        if let Err(e) = self.client.ensure_model_available().await {
            spinner.finish_and_clear();
            return Err(e);
        }
        
        spinner.set_message("Loading embedding cache...");

        // Try to load from cache first
        if let Ok(cache) = self.load_cache() {
            eprintln!("  Cache file found with {} embeddings (version: {})", cache.embeddings.len(), cache.version);
            
            if cache.embeddings.is_empty() {
                eprintln!("  ⚠️  Cache is empty - will rebuild index");
            }
            
            if cache.is_valid() && !cache.embeddings.is_empty() {
                spinner.set_message(format!("Validating {} cached embeddings...", cache.embeddings.len()));
                
                let (valid, invalid) = self.validate_cache_entries(&cache.embeddings)?;
                
                // Add valid entries to store and BM25 index
                for entry in &valid {
                    let chunk_path = format!("{}:{}-{}", entry.path.display(), entry.start_line, entry.end_line);
                    self.file_times.insert(entry.path.clone(), entry.modified);
                    self.store.add(PathBuf::from(&chunk_path), entry.embedding.clone(), entry.preview.clone());
                    
                    // Rebuild BM25 index from preview (approximation)
                    self.bm25.add_document(PathBuf::from(chunk_path), &entry.preview);
                }
                
                // Finalize BM25 index
                self.bm25.finalize();
                eprintln!("  BM25 index built with {} documents", self.bm25.len());
                
                eprintln!("  Total embeddings in store: {}", self.store.len());

                // Check for new files not in cache
                let all_files = self.collect_source_files()?;
                let cached_paths: std::collections::HashSet<_> = cache.embeddings.iter()
                    .map(|e| &e.path)
                    .collect();
                let new_files: Vec<_> = all_files.into_iter()
                    .filter(|f| !cached_paths.contains(f))
                    .collect();
                
                let new_count = new_files.len();
                let invalid_count = invalid.len();
                
                if !new_files.is_empty() {
                    eprintln!("  Found {} new files to embed", new_count);
                    spinner.set_message(format!("Embedding {} new files...", new_count));
                    self.embed_files(new_files, &spinner).await?;
                }

                if !invalid.is_empty() {
                    // Re-embed invalid files
                    spinner.set_message(format!("Re-embedding {} modified files...", invalid_count));
                    self.embed_files(invalid, &spinner).await?;
                    
                    spinner.finish_with_message(format!("✓ Loaded cache + updated {} files", invalid_count + new_count));
                } else if new_count > 0 {
                    spinner.finish_with_message(format!("✓ Loaded cache + added {} new files", new_count));
                } else {
                    spinner.finish_with_message(format!("✓ Loaded {} embeddings from cache", cache.embeddings.len()));
                }
                
                self.save_cache()?;
                return Ok(());
            }
            
            // Cache is valid but empty - fall through to rebuild
            eprintln!("  Cache is empty - falling through to rebuild");
        }

        // No valid cache - embed entire codebase
        eprintln!("  No valid cache found - building from scratch");
        spinner.set_message("Building embedding index for codebase...");
        let files = self.collect_source_files()?;
        
        eprintln!("  Found {} source files to embed", files.len());
        spinner.set_message(format!("Embedding {} source files...", files.len()));
        self.embed_files(files, &spinner).await?;
        
        eprintln!("  Embedded {} files total", self.store.len());
        spinner.finish_with_message(format!("✓ Indexed {} files with embeddings", self.store.len()));
        
        eprintln!("  Saving cache to disk...");
        self.save_cache()?;
        eprintln!("  ✓ Cache saved");
        
        Ok(())
    }

    /// Hybrid search combining BM25 keyword search and vector semantic search
    pub async fn search(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        eprintln!("  Hybrid search: {} embeddings, {} BM25 docs", self.store.len(), self.bm25.len());
        
        if self.store.is_empty() {
            eprintln!("  ⚠️  Vector store is empty - no results");
            return Ok(Vec::new());
        }
        
        // Run BM25 keyword search
        let bm25_results = self.bm25.search(query, top_k * 2);  // Get more for ranking
        eprintln!("  BM25 found {} keyword matches", bm25_results.len());
        
        // Run vector semantic search
        let query_embedding = self.client.embed(query).await?;
        let vector_results = self.store.search(&query_embedding, top_k * 2);
        eprintln!("  Vector found {} semantic matches", vector_results.len());
        
        // Combine results using adaptive weighted fusion
        let mut combined = self.reciprocal_rank_fusion(query, &bm25_results, &vector_results, top_k);
        
        // Build import graph for graph-based ranking
        let all_files: Vec<PathBuf> = combined.iter().map(|r| r.file_path.clone()).collect();
        let import_graph = self.build_import_graph(&all_files);
        
        // Apply graph-based boost
        self.apply_graph_boost(&mut combined, &import_graph);
        
        // Apply import-based boosting using preview content
        for result in &mut combined {
            let import_boost = Self::boost_by_imports(&result.preview, query);
            result.score *= import_boost;
        }
        
        // Re-sort after boosting
        combined.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        // Re-normalize after boosting
        if let Some(max_score) = combined.first().map(|r| r.score) {
            if max_score > 0.0 {
                for result in &mut combined {
                    result.score = result.score / max_score;
                }
            }
        }
        
        eprintln!("  Combined {} results using RRF + import boost", combined.len());
        if !combined.is_empty() {
            eprintln!("  Top scores: {:?}", combined.iter().take(5).map(|r| r.score).collect::<Vec<_>>());
        }
        
        // Filter by minimum similarity score
        let filtered: Vec<_> = combined.into_iter()
            .filter(|r| r.score >= MIN_SIMILARITY_SCORE)
            .collect();
        
        eprintln!("  After filtering (score >= {}): {} results", MIN_SIMILARITY_SCORE, filtered.len());
        
        Ok(filtered)
    }

    /// Check if file content has imports matching query terms
    fn boost_by_imports(content: &str, query: &str) -> f32 {
        let mut boost = 1.0;
        let query_terms: Vec<&str> = query.split_whitespace()
            .filter(|t| t.len() > 3)
            .collect();
        
        if query_terms.is_empty() {
            return boost;
        }
        
        // Extract import lines
        let imports: Vec<&str> = content
            .lines()
            .filter(|l| {
                let trimmed = l.trim();
                trimmed.starts_with("use ") || 
                trimmed.starts_with("import ") ||
                trimmed.starts_with("from ") ||
                trimmed.starts_with("require(")
            })
            .collect();
        
        // Check if imports match query terms
        for term in &query_terms {
            let term_lower = term.to_lowercase();
            if imports.iter().any(|i| i.to_lowercase().contains(&term_lower)) {
                boost += 0.2;
            }
        }
        
        boost.min(2.0)
    }

    /// Calculate file type and location boost
    fn calculate_file_boost(path: &Path) -> f32 {
        let path_str = path.to_str().unwrap_or("");
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        // Heavy penalty for test files
        if path_str.contains("/tests/") || path_str.contains("\\tests\\") {
            return 0.1;
        }
        
        // Heavy penalty for benchmark files
        if path_str.contains("/benches/") || path_str.contains("\\benches\\") ||
           path_str.contains("/benchmarks/") || path_str.contains("\\benchmarks\\") {
            return 0.1;
        }
        
        let mut type_boost = match ext {
            "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "java" | "c" | "cpp" |
            "h" | "hpp" | "go" | "rb" | "php" | "cs" | "swift" | "kt" | "scala" => 1.7,
            "toml" | "yaml" | "yml" | "json" | "xml" => 0.5,
            "md" | "txt" => 0.1,  // Heavy penalty for all documentation
            _ => 1.0,
        };
        
        // Boost module entry points
        if path_str.ends_with("/lib.rs") || path_str.ends_with("\\lib.rs") {
            type_boost *= 1.3;  // Entry points are important
        } else if path_str.ends_with("/mod.rs") || path_str.ends_with("\\mod.rs") {
            type_boost *= 1.2;  // Module definitions
        }
        
        let location_boost = if path_str.contains("/src/") || path_str.contains("\\src\\") {
            1.3
        } else if path_str.contains("/docs/") || path_str.contains("\\docs\\") ||
                  path_str.contains("/examples/") || path_str.contains("\\examples\\") {
            0.5
        } else {
            1.0
        };
        
        type_boost * location_boost
    }

    /// Calculate query-file alignment based on keyword matching
    fn calculate_query_file_alignment(query: &str, file_path: &Path, preview: &str) -> f32 {
        let mut alignment = 1.0;
        let query_lower = query.to_lowercase();
        
        // Extract query keywords (words longer than 3 chars)
        let keywords: Vec<&str> = query_lower
            .split_whitespace()
            .filter(|w| w.len() > 3 && !matches!(*w, "the" | "and" | "for" | "with" | "from" | "that" | "this"))
            .collect();
        
        if keywords.is_empty() {
            return alignment;
        }
        
        // Check if filename contains query keywords
        let filename = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        for keyword in &keywords {
            if filename.contains(keyword) {
                alignment *= 1.4;  // Filename match is strong signal
            }
        }
        
        // Check parent directory names
        if let Some(parent) = file_path.parent() {
            let parent_str = parent.to_str().unwrap_or("").to_lowercase();
            for keyword in &keywords {
                if parent_str.contains(keyword) {
                    alignment *= 1.2;  // Directory match is good signal
                }
            }
        }
        
        // Keyword density in preview
        let preview_lower = preview.to_lowercase();
        let keyword_count = keywords.iter()
            .filter(|k| preview_lower.contains(*k))
            .count();
        
        if keyword_count > 0 {
            let density_boost = 1.0 + (keyword_count as f32 * 0.1);
            alignment *= density_boost.min(1.5);  // Cap at 1.5x
        }
        
        alignment
    }

    /// Calculate pattern-based importance boost for code structure
    fn calculate_pattern_boost(preview: &str) -> f32 {
        let mut boost = 1.0;
        
        // Implementation pattern detection
        let has_impl = preview.contains("impl ") || preview.contains("impl<");
        let has_trait = preview.contains("trait ");
        let has_struct = preview.contains("pub struct") || preview.contains("pub enum");
        let has_main_fn = preview.contains("fn main(") || preview.contains("pub fn new(");
        
        if has_impl && has_struct {
            boost *= 1.3;  // Core implementation file
        }
        
        if has_trait {
            boost *= 1.2;  // Trait definitions are important
        }
        
        if has_main_fn {
            boost *= 1.25;  // Entry point functions
        }
        
        // Count pub items (public API)
        let pub_count = preview.matches("pub fn").count() 
            + preview.matches("pub struct").count()
            + preview.matches("pub enum").count();
        
        if pub_count > 5 {
            boost *= 1.2;  // Rich public API
        }
        
        // Module-level documentation at start
        if preview.trim_start().starts_with("//!") {
            boost *= 1.15;  // Module docs indicate important file
        }
        
        boost
    }

    /// Build import graph from Rust source files using rust-analyzer
    fn build_import_graph(&self, files: &[PathBuf]) -> HashMap<PathBuf, Vec<PathBuf>> {
        // Try to use rust-analyzer backend for accurate import resolution
        if let Ok(backend) = self.try_get_rust_backend() {
            if let Ok(graph) = backend.build_import_graph(files) {
                return graph;
            }
        }
        
        // Fallback: return empty graph if rust-analyzer not available
        // This is acceptable since graph ranking is a bonus feature
        HashMap::new()
    }
    
    /// Try to get or create a Rust backend for the project
    fn try_get_rust_backend(&self) -> Result<rust_backend::RustBackend> {
        use rust_backend::RustBackend;
        
        let mut backend = RustBackend::new();
        backend.initialize(&self.project_root)?;
        Ok(backend)
    }
    
    /// Apply graph-based boost to results
    fn apply_graph_boost(&self, results: &mut [SearchResult], graph: &HashMap<PathBuf, Vec<PathBuf>>) {
        // Build reverse graph (who imports this file)
        let mut reverse_graph: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
        for (file, imports) in graph {
            for imported in imports {
                reverse_graph.entry(imported.clone())
                    .or_default()
                    .push(file.clone());
            }
        }
        
        // Boost files based on graph relationships
        for result in results.iter_mut() {
            let mut graph_boost = 1.0;
            
            // Boost if many files import this (central/important)
            if let Some(importers) = reverse_graph.get(&result.file_path) {
                let import_count = importers.len();
                if import_count > 5 {
                    graph_boost *= 1.3;  // Heavily imported = important
                } else if import_count > 2 {
                    graph_boost *= 1.15;  // Moderately imported
                }
            }
            
            // Boost if this file imports many others (coordinator/orchestrator)
            if let Some(imports) = graph.get(&result.file_path) {
                let import_count = imports.len();
                if import_count > 10 {
                    graph_boost *= 1.2;  // Orchestrator file
                }
            }
            
            result.score *= graph_boost;
        }
    }

    /// Calculate chunk quality boost based on content
    fn calculate_chunk_quality(preview: &str) -> f32 {
        let mut boost = 1.0;
        
        // Boost chunks with definitions
        if preview.contains("pub struct") || preview.contains("pub enum") || preview.contains("pub trait") {
            boost *= 1.4;
        }
        
        if preview.contains("pub fn") || preview.contains("pub async fn") {
            boost *= 1.3;
        }
        
        // Boost module-level documentation
        if preview.trim_start().starts_with("///") || preview.trim_start().starts_with("//!") {
            boost *= 1.2;
        }
        
        // Penalize chunks that are mostly comments or whitespace
        let non_whitespace_lines = preview.lines()
            .filter(|line| {
                let trimmed = line.trim();
                !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.starts_with("/*")
            })
            .count();
        
        if non_whitespace_lines < 3 {
            boost *= 0.5;  // Mostly empty or comments
        }
        
        boost
    }

    /// Detect query intent from keywords
    fn detect_query_intent(query: &str) -> &'static str {
        let query_lower = query.to_lowercase();
        
        if query_lower.starts_with("how") || query_lower.contains(" work") {
            "explanation"
        } else if query_lower.starts_with("implement") || query_lower.starts_with("add") {
            "implementation"
        } else if query_lower.starts_with("fix") || query_lower.starts_with("debug") || query_lower.starts_with("where") {
            "debugging"
        } else {
            "general"
        }
    }

    /// Calculate adaptive weights based on query characteristics
    fn calculate_adaptive_weights(query: &str) -> (f32, f32) {
        // Detect special tokens that indicate exact matching is important
        let has_special_tokens = query.contains("::") || query.contains("--") || query.contains("#[");
        let intent = Self::detect_query_intent(query);
        
        if has_special_tokens {
            // Favor BM25 for exact matches
            (0.7, 0.3)
        } else {
            match intent {
                "explanation" => (0.3, 0.7),  // Favor semantics for "how does X work"
                "implementation" => (0.5, 0.5),  // Balanced for "implement X"
                "debugging" => (0.6, 0.4),  // Favor keywords for "fix/where is X"
                _ => (0.4, 0.6),  // Default
            }
        }
    }

    /// Combine BM25 keyword scores with vector semantic scores using weighted normalization
    fn reciprocal_rank_fusion(
        &self,
        query: &str,
        bm25_results: &[(PathBuf, f32)],
        vector_results: &[SearchResult],
        top_k: usize,
    ) -> Vec<SearchResult> {
        let (bm25_weight, vector_weight) = Self::calculate_adaptive_weights(query);

        let mut bm25_scores: HashMap<PathBuf, f32> = HashMap::new();
        let mut vector_scores: HashMap<PathBuf, f32> = HashMap::new();
        let mut previews: HashMap<PathBuf, String> = HashMap::new();
        let mut paths: HashSet<PathBuf> = HashSet::new();

        let mut max_bm25 = 0.0f32;
        for (path, score) in bm25_results.iter() {
            if *score > 0.0 {
                bm25_scores.insert(path.clone(), *score);
                if *score > max_bm25 {
                    max_bm25 = *score;
                }
                paths.insert(path.clone());
            }
        }

        let mut max_vector = 0.0f32;
        for result in vector_results {
            if result.score > 0.0 {
                vector_scores.insert(result.file_path.clone(), result.score);
                if result.score > max_vector {
                    max_vector = result.score;
                }
                paths.insert(result.file_path.clone());
            }
            previews.insert(result.file_path.clone(), result.preview.clone());
        }

        let mut combined: Vec<SearchResult> = paths
            .into_iter()
            .map(|path| {
                let bm25_raw = bm25_scores.get(&path).copied().unwrap_or(0.0);
                let vector_raw = vector_scores.get(&path).copied().unwrap_or(0.0);

                let bm25_normalized = if max_bm25 > 0.0 { bm25_raw / max_bm25 } else { 0.0 };
                let vector_normalized = if max_vector > 0.0 { vector_raw / max_vector } else { 0.0 };

                // Apply minimum BM25 threshold - weak matches don't contribute
                // Tuned threshold: 0.75 balances precision and recall
                let mut bm25_contribution = if bm25_raw >= 0.75 {
                    bm25_normalized * bm25_weight
                } else {
                    0.0
                };
                
                // Exact match bonus: check if preview contains exact query terms
                if bm25_contribution > 0.0 {
                    let preview = previews.get(&path).map(|s| s.to_lowercase()).unwrap_or_default();
                    let query_lower = query.to_lowercase();
                    
                    // Check for special tokens (--flags, ::paths, #[attributes])
                    let special_tokens: Vec<&str> = query_lower.split_whitespace()
                        .filter(|t| t.contains("--") || t.contains("::") || t.contains("#["))
                        .collect();
                    
                    for token in special_tokens {
                        if preview.contains(token) {
                            bm25_contribution *= 1.5;  // Exact match bonus
                            break;
                        }
                    }
                }
                
                let vector_contribution = vector_normalized * vector_weight;

                let preview = previews.get(&path).cloned().unwrap_or_default();
                let file_boost = Self::calculate_file_boost(&path);
                let query_alignment = Self::calculate_query_file_alignment(query, &path, &preview);
                let pattern_boost = Self::calculate_pattern_boost(&preview);
                let chunk_quality = Self::calculate_chunk_quality(&preview);
                let combined_score = (bm25_contribution + vector_contribution) * file_boost * query_alignment * pattern_boost * chunk_quality;

                SearchResult {
                    file_path: path.clone(),
                    score: combined_score,
                    preview,
                    bm25_score: if bm25_contribution > 0.0 { Some(bm25_contribution) } else { None },
                    vector_score: if vector_contribution > 0.0 { Some(vector_contribution) } else { None },
                }
            })
            .collect();

        combined.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        if let Some(max_score) = combined.first().map(|r| r.score) {
            if max_score > 0.0 {
                for result in &mut combined {
                    result.score = result.score / max_score;
                    if let Some(b) = result.bm25_score.as_mut() {
                        *b = *b / max_score;
                    }
                    if let Some(v) = result.vector_score.as_mut() {
                        *v = *v / max_score;
                    }
                }
            }
        }

        combined.truncate(top_k);

        combined
    }

    /// Collect all source files in the project
    fn collect_source_files(&self) -> Result<Vec<PathBuf>> {
        use ignore::WalkBuilder;
        
        let mut files = Vec::new();
        
        let walker = WalkBuilder::new(&self.project_root)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(false)
            .build();

        for entry in walker.filter_map(std::result::Result::ok) {
            let path = entry.path();
            
            if entry.file_type().map_or(false, |ft| ft.is_file()) && is_source_file(path) {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    /// Embed a batch of files (chunked)
    async fn embed_files(&mut self, files: Vec<PathBuf>, spinner: &ProgressBar) -> Result<()> {
        const BATCH_SIZE: usize = 10;
        let total_files = files.len();
        let mut processed_files = 0;
        let mut total_chunks = 0;

        for file_batch in files.chunks(BATCH_SIZE) {
            let mut tasks = JoinSet::new();
            
            for file_path in file_batch {
                let path = file_path.clone();
                let client = EmbeddingClient::new();
                
                tasks.spawn(async move {
                    let content = match fs::read_to_string(&path) {
                        Ok(c) => c,
                        Err(e) => {
                            eprintln!("Warning: Failed to read {}: {}", path.display(), e);
                            return Vec::new();
                        }
                    };

                    // Skip empty files
                    if content.trim().is_empty() {
                        return Vec::new();
                    }

                    // Chunk the file
                    let chunks = chunk_file(&path, &content);
                    let mut chunk_results = Vec::new();
                    
                    for chunk in chunks {
                        let preview = generate_preview(&chunk.content, 200);
                        
                        match client.embed(&chunk.content).await {
                            Ok(embedding) => {
                                chunk_results.push((path.clone(), chunk, embedding, preview));
                            }
                            Err(e) => {
                                eprintln!("Warning: Failed to embed chunk in {}: {}", path.display(), e);
                            }
                        }
                    }
                    
                    chunk_results
                });
            }

            // Collect results
            while let Some(result) = tasks.join_next().await {
                match result {
                    Ok(chunk_results) => {
                        if !chunk_results.is_empty() {
                            let file_path = &chunk_results[0].0;
                            
                            // Track file modification time
                            if let Ok(metadata) = fs::metadata(file_path) {
                                if let Ok(modified) = metadata.modified() {
                                    self.file_times.insert(file_path.clone(), modified);
                                }
                            }
                            
                            for (path, chunk, embedding, preview) in chunk_results {
                                let chunk_path = format!("{}:{}-{}", path.display(), chunk.start_line, chunk.end_line);

                                // Add to vector store
                                self.store.add(PathBuf::from(&chunk_path), embedding, preview);
                                
                                // Add to BM25 index
                                self.bm25.add_document(PathBuf::from(chunk_path), &chunk.content);
                                
                                total_chunks += 1;
                            }
                            
                            processed_files += 1;
                            spinner.set_message(format!("Embedding files... {}/{} ({} chunks)", processed_files, total_files, total_chunks));
                        }
                    }
                    Err(e) => {
                        eprintln!("    Task error: {}", e);
                    }
                }
            }
        }

        // Finalize BM25 index (compute IDF scores)
        self.bm25.finalize();
        eprintln!("  BM25 index finalized with {} documents", self.bm25.len());

        Ok(())
    }

    /// Validate cache entries and return (valid, invalid)
    fn validate_cache_entries(&self, entries: &[CachedEmbedding]) -> Result<(Vec<CachedEmbedding>, Vec<PathBuf>)> {
        let mut valid = Vec::new();
        let mut invalid = Vec::new();

        for entry in entries {
            // Check if file still exists
            if !entry.path.exists() {
                continue;
            }

            // Check if file was modified
            match fs::metadata(&entry.path) {
                Ok(metadata) => {
                    match metadata.modified() {
                        Ok(modified) => {
                            if modified > entry.modified {
                                invalid.push(entry.path.clone());
                            } else {
                                valid.push(entry.clone());
                            }
                        }
                        Err(_) => invalid.push(entry.path.clone()),
                    }
                }
                Err(_) => continue, // File doesn't exist anymore
            }
        }

        Ok((valid, invalid))
    }

    /// Load cache from disk
    fn load_cache(&self) -> Result<VectorCache> {
        let data = fs::read(&self.cache_path)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to read cache: {e}")))?;
        
        bincode::deserialize(&data)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to deserialize cache: {e}")))
    }

    /// Save cache to disk
    fn save_cache(&self) -> Result<()> {
        // Create cache directory if needed
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| agentic_core::Error::Other(format!("Failed to create cache dir: {e}")))?;
        }

        // Build cache entries from store
        let mut cache = VectorCache::new();
        
        for entry in self.store.iter() {
            // Parse chunk path: "file_path:start-end"
            let path_str = entry.path.display().to_string();
            if let Some((file_part, range_part)) = path_str.rsplit_once(':') {
                if let Some((start_str, end_str)) = range_part.split_once('-') {
                    if let (Ok(start), Ok(end)) = (start_str.parse(), end_str.parse()) {
                        let file_path = PathBuf::from(file_part);
                        let modified = self.file_times.get(&file_path)
                            .copied()
                            .unwrap_or_else(SystemTime::now);
                        
                        cache.embeddings.push(CachedEmbedding {
                            path: file_path,
                            chunk_id: String::from("chunk"),  // We don't store this separately
                            start_line: start,
                            end_line: end,
                            embedding: entry.embedding,
                            preview: entry.preview,
                            modified,
                        });
                        continue;
                    }
                }
            }
            
            // Fallback for non-chunked entries (shouldn't happen)
            eprintln!("Warning: Could not parse chunk path: {}", path_str);
        }
        
        let data = bincode::serialize(&cache)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to serialize cache: {e}")))?;
        
        fs::write(&self.cache_path, data)
            .map_err(|e| agentic_core::Error::Other(format!("Failed to write cache: {e}")))?;

        Ok(())
    }

    /// Get the number of indexed files
    #[must_use]
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }
}
