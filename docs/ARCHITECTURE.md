# Architecture - Detailed Module Design

## Multi-Crate Structure

The project is organized as a Cargo workspace with separate crates for each major component. This provides:
- **Clear boundaries** between components
- **Independent testing** of each crate
- **Parallel compilation** for faster builds
- **Easy experimentation** by swapping implementations

```
merlin/
├── crates/
│   ├── merlin-core/           # Core types, traits, and errors
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs        # Query, Response, Context, TokenUsage
│   │       ├── traits.rs       # ModelProvider trait
│   │       └── error.rs        # Error types and Result
│   │
│   ├── merlin-providers/      # Model provider implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── anthropic.rs    # Claude Sonnet integration
│   │       ├── groq.rs         # Groq API (Phase 3)
│   │       ├── gemini.rs       # Gemini Flash (Phase 3)
│   │       └── local.rs        # Ollama integration (Phase 4)
│   │
│   ├── merlin-context/        # Context building and indexing
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── builder.rs      # Context builder
│   │       └── index/          # Codebase indexing (Phase 1)
│   │           ├── mod.rs
│   │           ├── symbol_table.rs
│   │           ├── dependency.rs
│   │           └── embeddings.rs
│   │
│   ├── merlin-routing/        # Routing engine (Phase 3)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── classifier.rs   # Query complexity classifier
│   │       ├── router.rs       # Model tier router
│   │       └── escalation.rs   # Escalation logic
│   │
│   └── merlin-cli/            # CLI application
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs         # Entry point
│           ├── cli/
│           │   ├── mod.rs
│           │   └── commands.rs # CLI commands
│           └── config/
│               └── mod.rs      # Configuration
│
├── docs/                       # Documentation
├── tests/                      # Integration tests
└── Cargo.toml                  # Workspace configuration
```

### Crate Dependencies

```
merlin-cli
├── merlin-core
├── merlin-providers
│   └── merlin-core
├── merlin-context
│   └── merlin-core
└── merlin-routing (Phase 3+)
    ├── merlin-core
    └── merlin-providers
```

### Phase 0 (MVP) Crates

Currently implemented:
- ✅ **merlin-core**: Core types and traits
- ✅ **merlin-providers**: Anthropic provider only
- ✅ **merlin-context**: Basic context builder
- ✅ **merlin-cli**: CLI interface

### Future Crates (Later Phases)

- **merlin-routing** (Phase 3): Query classification and model routing
- **agentic-observability** (Phase 5): Metrics and cost tracking

## Core Traits

### ModelProvider Trait

All model providers implement this trait for uniform interface:

```rust
use async_trait::async_trait;
use crate::core::{Query, Response, Context, ModelConfig};

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Provider name for logging/metrics
    fn name(&self) -> &str;
    
    /// Check if provider is available
    async fn is_available(&self) -> bool;
    
    /// Generate response for query with context
    async fn generate(
        &self,
        query: &Query,
        context: &Context,
        config: &ModelConfig,
    ) -> Result<Response>;
    
    /// Estimate cost before calling (tokens * rate)
    fn estimate_cost(&self, context: &Context) -> f64;
    
    /// Get provider tier (for routing)
    fn tier(&self) -> ProviderTier;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderTier {
    Local = 0,      // Free, fast
    FreeTier = 1,   // Groq free tier
    Budget = 2,     // Gemini Flash
    Premium = 3,    // Sonnet 4.5
}
```

### ContextBuilder Trait

```rust
pub trait ContextBuilder: Send + Sync {
    /// Build minimal context for query
    fn build_context(
        &self,
        query: &Query,
        strategy: ContextStrategy,
    ) -> Result<Context>;
    
    /// Get relevant files for query
    fn find_relevant_files(&self, query: &Query) -> Vec<PathBuf>;
    
    /// Resolve dependencies for files
    fn resolve_dependencies(&self, files: &[PathBuf]) -> Vec<PathBuf>;
}

pub enum ContextStrategy {
    Minimal,        // Just mentioned symbols
    Dependencies,   // Include direct deps
    Full,           // Include transitive deps (rare)
}
```

### CodebaseIndex Trait

```rust
pub trait CodebaseIndex: Send + Sync {
    /// Find definition of symbol
    fn find_definition(&self, symbol: &str) -> Option<Location>;
    
    /// Find all references to symbol
    fn find_references(&self, symbol: &str) -> Vec<Location>;
    
    /// Get file dependencies
    fn get_dependencies(&self, file: &Path) -> Vec<PathBuf>;
    
    /// Search by semantic similarity (optional)
    fn semantic_search(&self, query: &str, top_k: usize) -> Vec<Location>;
    
    /// Rebuild index for path
    fn rebuild(&mut self, path: &Path) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
}
```

## Core Types

```rust
/// User query with metadata
#[derive(Debug, Clone)]
pub struct Query {
    pub text: String,
    pub conversation_id: Option<String>,
    pub files_context: Vec<PathBuf>,  // Files user has open
    pub cursor_position: Option<Location>,
}

/// Model response with metadata
#[derive(Debug, Clone)]
pub struct Response {
    pub text: String,
    pub confidence: f64,
    pub tool_calls: Vec<ToolCall>,
    pub tokens_used: TokenUsage,
    pub provider: String,
    pub latency_ms: u64,
}

/// Context package sent to model
#[derive(Debug, Clone)]
pub struct Context {
    pub files: Vec<FileContext>,
    pub system_prompt: String,
    pub conversation_history: Vec<Message>,
}

#[derive(Debug, Clone)]
pub struct FileContext {
    pub path: PathBuf,
    pub content: String,
    pub relevant_lines: Option<(usize, usize)>,  // Focus on specific lines
}

#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// Tool calls extracted from response
#[derive(Debug, Clone)]
pub enum ToolCall {
    EditFile { path: PathBuf, edits: Vec<Edit> },
    CreateFile { path: PathBuf, content: String },
    SearchCode { query: String },
    RunCommand { command: String },
}

#[derive(Debug, Clone)]
pub struct Edit {
    pub start_line: usize,
    pub end_line: usize,
    pub new_content: String,
}
```

## Routing Engine Design

```rust
pub struct RoutingEngine {
    classifier: QueryClassifier,
    config: RoutingConfig,
}

impl RoutingEngine {
    pub fn route(&self, query: &Query) -> RoutingDecision {
        let complexity = self.classifier.classify(query);
        let context_strategy = self.determine_context_strategy(complexity);
        let provider_tier = self.select_provider_tier(complexity);
        
        RoutingDecision {
            complexity,
            context_strategy,
            provider_tier,
            max_retries: self.config.max_retries,
        }
    }
    
    fn determine_context_strategy(&self, complexity: Complexity) -> ContextStrategy {
        match complexity {
            Complexity::Simple => ContextStrategy::Minimal,
            Complexity::Medium => ContextStrategy::Dependencies,
            Complexity::Complex => ContextStrategy::Dependencies,
        }
    }
    
    fn select_provider_tier(&self, complexity: Complexity) -> ProviderTier {
        match complexity {
            Complexity::Simple => ProviderTier::Local,
            Complexity::Medium => ProviderTier::Local,  // Try local first
            Complexity::Complex => ProviderTier::FreeTier,  // Start with Groq
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Complexity {
    Simple,   // Searches, lookups, simple questions
    Medium,   // Single-file edits, refactors
    Complex,  // Multi-file changes, architecture
}

pub struct RoutingDecision {
    pub complexity: Complexity,
    pub context_strategy: ContextStrategy,
    pub provider_tier: ProviderTier,
    pub max_retries: usize,
}
```

## Query Classifier

Uses pattern matching and heuristics:

```rust
pub struct QueryClassifier {
    simple_patterns: Vec<Regex>,
    complex_patterns: Vec<Regex>,
}

impl QueryClassifier {
    pub fn classify(&self, query: &Query) -> Complexity {
        let text = query.text.to_lowercase();
        
        // Simple queries
        if self.is_simple(&text) {
            return Complexity::Simple;
        }
        
        // Complex queries
        if self.is_complex(&text) {
            return Complexity::Complex;
        }
        
        // Default to medium
        Complexity::Medium
    }
    
    fn is_simple(&self, text: &str) -> bool {
        const SIMPLE_KEYWORDS: &[&str] = &[
            "find", "search", "where is", "show", "list",
            "what is", "who", "when", "view"
        ];
        
        SIMPLE_KEYWORDS.iter().any(|kw| text.contains(kw))
    }
    
    fn is_complex(&self, text: &str) -> bool {
        const COMPLEX_KEYWORDS: &[&str] = &[
            "design", "architecture", "refactor all", "optimize",
            "explain why", "rewrite", "migrate", "implement"
        ];
        
        COMPLEX_KEYWORDS.iter().any(|kw| text.contains(kw))
            || text.split_whitespace().count() > 20  // Long queries often complex
    }
}
```

## Model Router

Manages multiple providers with escalation:

```rust
pub struct ModelRouter {
    providers: HashMap<ProviderTier, Box<dyn ModelProvider>>,
    metrics: Arc<MetricsCollector>,
}

impl ModelRouter {
    pub async fn route_request(
        &self,
        query: &Query,
        context: &Context,
        decision: &RoutingDecision,
    ) -> Result<Response> {
        let mut current_tier = decision.provider_tier;
        
        loop {
            let provider = self.providers.get(&current_tier)
                .ok_or_else(|| Error::ProviderNotAvailable(current_tier))?;
            
            // Check if provider is available
            if !provider.is_available().await {
                current_tier = self.escalate_tier(current_tier)?;
                continue;
            }
            
            // Try the provider
            match self.try_provider(provider, query, context).await {
                Ok(response) if response.confidence >= 0.85 => {
                    self.metrics.record_success(provider.name(), current_tier);
                    return Ok(response);
                }
                Ok(response) if current_tier < ProviderTier::Premium => {
                    // Low confidence, try escalating
                    self.metrics.record_escalation(provider.name(), current_tier);
                    current_tier = self.escalate_tier(current_tier)?;
                }
                Ok(response) => {
                    // Already at premium tier, return with disclaimer
                    return Ok(response);
                }
                Err(error) if error.is_retryable() => {
                    // Network error, etc - retry same tier
                    tokio::time::sleep(Duration::from_millis(1000)).await;
                    continue;
                }
                Err(_) => {
                    // Non-retryable error, escalate
                    current_tier = self.escalate_tier(current_tier)?;
                }
            }
        }
    }
    
    fn escalate_tier(&self, current: ProviderTier) -> Result<ProviderTier> {
        match current {
            ProviderTier::Local => Ok(ProviderTier::FreeTier),
            ProviderTier::FreeTier => Ok(ProviderTier::Budget),
            ProviderTier::Budget => Ok(ProviderTier::Premium),
            ProviderTier::Premium => Err(Error::NoHigherTier),
        }
    }
}
```

## Context Builder Implementation

```rust
pub struct MinimalContextBuilder {
    index: Arc<dyn CodebaseIndex>,
    config: ContextConfig,
}

impl ContextBuilder for MinimalContextBuilder {
    fn build_context(
        &self,
        query: &Query,
        strategy: ContextStrategy,
    ) -> Result<Context> {
        let mut files = Vec::new();
        
        // 1. Extract mentioned symbols from query
        let symbols = self.extract_symbols(&query.text);
        
        // 2. Find definitions
        for symbol in &symbols {
            if let Some(location) = self.index.find_definition(symbol) {
                files.push(location.file);
            }
        }
        
        // 3. Add files user has open
        files.extend(query.files_context.clone());
        
        // 4. Resolve dependencies if needed
        if matches!(strategy, ContextStrategy::Dependencies) {
            let deps: Vec<_> = files.iter()
                .flat_map(|f| self.index.get_dependencies(f))
                .collect();
            files.extend(deps);
        }
        
        // 5. Limit to max_files
        files.truncate(self.config.max_files);
        
        // 6. Build file contexts
        let file_contexts: Vec<_> = files.into_iter()
            .filter_map(|path| self.read_file_context(&path).ok())
            .collect();
        
        Ok(Context {
            files: file_contexts,
            system_prompt: self.build_system_prompt(),
            conversation_history: vec![],
        })
    }
    
    fn extract_symbols(&self, text: &str) -> Vec<String> {
        // Simple regex-based extraction
        // In production: could use NLP or local model
        let re = Regex::new(r"\b[A-Z][a-zA-Z0-9_]*\b|\b[a-z_][a-z0-9_]*\b").unwrap();
        re.find_iter(text)
            .map(|m| m.as_str().to_string())
            .filter(|s| s.len() > 2)  // Filter short words
            .collect()
    }
}

#[derive(Clone)]
pub struct ContextConfig {
    pub max_files: usize,
    pub max_tokens: usize,
    pub include_tests: bool,
}
```

## Observability

```rust
use std::sync::atomic::{AtomicU64, Ordering};

pub struct MetricsCollector {
    // Cost tracking
    total_input_tokens: AtomicU64,
    total_output_tokens: AtomicU64,
    total_cache_read_tokens: AtomicU64,
    total_cache_write_tokens: AtomicU64,
    
    // Per-provider metrics
    provider_metrics: DashMap<String, ProviderMetrics>,
    
    // Performance tracking
    request_count: AtomicU64,
    success_count: AtomicU64,
    escalation_count: AtomicU64,
}

#[derive(Default)]
struct ProviderMetrics {
    calls: AtomicU64,
    successes: AtomicU64,
    escalations: AtomicU64,
    total_latency_ms: AtomicU64,
    total_cost_usd: AtomicU64,  // In micro-dollars
}

impl MetricsCollector {
    pub fn record_request(&self, response: &Response) {
        self.total_input_tokens.fetch_add(response.tokens_used.input, Ordering::Relaxed);
        self.total_output_tokens.fetch_add(response.tokens_used.output, Ordering::Relaxed);
        self.total_cache_read_tokens.fetch_add(response.tokens_used.cache_read, Ordering::Relaxed);
        self.total_cache_write_tokens.fetch_add(response.tokens_used.cache_write, Ordering::Relaxed);
        
        self.request_count.fetch_add(1, Ordering::Relaxed);
        if response.confidence >= 0.85 {
            self.success_count.fetch_add(1, Ordering::Relaxed);
        }
        
        let entry = self.provider_metrics.entry(response.provider.clone()).or_default();
        entry.calls.fetch_add(1, Ordering::Relaxed);
        entry.total_latency_ms.fetch_add(response.latency_ms, Ordering::Relaxed);
    }
    
    pub fn daily_report(&self) -> DailyReport {
        let input = self.total_input_tokens.load(Ordering::Relaxed);
        let output = self.total_output_tokens.load(Ordering::Relaxed);
        let cache_read = self.total_cache_read_tokens.load(Ordering::Relaxed);
        let cache_write = self.total_cache_write_tokens.load(Ordering::Relaxed);
        
        DailyReport {
            total_requests: self.request_count.load(Ordering::Relaxed),
            success_rate: self.success_count.load(Ordering::Relaxed) as f64 
                / self.request_count.load(Ordering::Relaxed) as f64,
            total_cost_usd: self.calculate_total_cost(input, output, cache_read, cache_write),
            tokens: TokenUsage { input, output, cache_read, cache_write },
        }
    }
}
```

## Configuration

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub routing: RoutingConfig,
    pub context: ContextConfig,
    pub local_models: LocalModelsConfig,
    pub providers: ProvidersConfig,
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RoutingConfig {
    pub enable_local: bool,
    pub enable_groq: bool,
    pub enable_gemini: bool,
    pub enable_sonnet: bool,
    pub confidence_threshold: f64,
    pub max_retries: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LocalModelsConfig {
    pub router_model: String,
    pub coder_model: String,
    pub ollama_url: String,
    pub device: Device,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum Device {
    Cuda,
    Cpu,
    Metal,  // For macOS
}
```

## Testing Utilities

```rust
// Mock provider for testing
pub struct MockProvider {
    responses: Vec<Response>,
    call_count: Arc<AtomicUsize>,
}

#[async_trait]
impl ModelProvider for MockProvider {
    fn name(&self) -> &str { "mock" }
    
    async fn is_available(&self) -> bool { true }
    
    async fn generate(
        &self,
        _query: &Query,
        _context: &Context,
        _config: &ModelConfig,
    ) -> Result<Response> {
        let idx = self.call_count.fetch_add(1, Ordering::Relaxed);
        Ok(self.responses[idx % self.responses.len()].clone())
    }
    
    fn estimate_cost(&self, _context: &Context) -> f64 { 0.0 }
    fn tier(&self) -> ProviderTier { ProviderTier::Local }
}

// Test helpers
pub mod test_helpers {
    pub fn create_test_query(text: &str) -> Query {
        Query {
            text: text.to_string(),
            conversation_id: None,
            files_context: vec![],
            cursor_position: None,
        }
    }
    
    pub fn create_test_context() -> Context {
        Context {
            files: vec![],
            system_prompt: String::new(),
            conversation_history: vec![],
        }
    }
}
```

## Dependencies (Cargo.toml)

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
async-trait = "0.1"

# HTTP clients
reqwest = { version = "0.12", features = ["json"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Error handling
anyhow = "1"
thiserror = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Concurrency
dashmap = "5"

# Regex
regex = "1"

# CLI (MVP)
clap = { version = "4", features = ["derive"] }

# Code parsing (for indexing)
tree-sitter = "0.20"
tree-sitter-rust = "0.20"

# Optional: embeddings for semantic search
# fastembed = { version = "3", optional = true }
```

See `PHASES.md` for incremental implementation plan.

