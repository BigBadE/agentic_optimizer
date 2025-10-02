# Implementation Phases

## Overview

This document outlines the phased implementation of the Agentic Optimizer, starting with a minimal viable product (MVP) and progressively adding optimization strategies. Each phase is designed to be independently testable and deployable.

**Timeline**: 4-5 weeks total
**Approach**: Ship early, optimize incrementally

---

## Phase 0: MVP - Basic Agent (Week 1, Days 1-3)

**Goal**: Get a working agent with Sonnet-only, no optimizations

**Features**:
- CLI interface for user queries
- Direct Sonnet 4.5 API calls
- Basic file reading/editing
- Simple error handling

**Modules to Implement**:
```
src/
├── main.rs              # CLI entry point
├── core/
│   ├── types.rs         # Query, Response, Context
│   └── error.rs         # Error types
├── providers/
│   ├── traits.rs        # ModelProvider trait
│   └── anthropic.rs     # Claude Sonnet integration
└── cli/
    └── commands.rs      # CLI commands
```

**Key Code**:

```rust
// main.rs
use clap::Parser;

#[derive(Parser)]
struct Args {
    /// The query to send to the agent
    query: String,
    
    /// Project root directory
    #[arg(short, long, default_value = ".")]
    project: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let provider = AnthropicProvider::new()?;
    
    // Build simple context (all files in project, naive approach)
    let context = build_full_context(&args.project)?;
    
    let response = provider.generate(&args.query, &context).await?;
    println!("{}", response.text);
    
    Ok(())
}
```

**Success Criteria**:
- [x] Can send query and get Sonnet response
- [x] Can read codebase files
- [x] Basic error handling works
- [x] CLI is functional

**Expected Cost**: $15-20/day (baseline, no optimization)
**Time Estimate**: 3 days

---

## Phase 1: Context Optimization (Week 1, Days 4-7)

**Goal**: Reduce cache operations by 60% through intelligent context selection

**New Features**:
- Codebase indexing (symbol table + dependency graph)
- Minimal context builder
- Query-based file selection
- Context caching

**New Modules**:
```
src/context/
├── builder.rs           # MinimalContextBuilder
├── index/
│   ├── symbol_table.rs  # Index functions/structs/modules
│   └── dependency.rs    # Dependency graph
└── cache.rs             # Context caching
```

**Implementation Steps**:

### Step 1.1: Symbol Table Indexer
```rust
// context/index/symbol_table.rs
use tree_sitter::{Parser, Language};

pub struct SymbolTable {
    symbols: HashMap<String, Vec<Location>>,
}

impl SymbolTable {
    pub fn build(project_root: &Path) -> Result<Self> {
        let mut symbols = HashMap::new();
        
        // Walk all Rust files
        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_entry(|e| !is_ignored(e))
        {
            let entry = entry?;
            if entry.path().extension() == Some(OsStr::new("rs")) {
                self.index_file(entry.path(), &mut symbols)?;
            }
        }
        
        Ok(SymbolTable { symbols })
    }
    
    fn index_file(&self, path: &Path, symbols: &mut HashMap<String, Vec<Location>>) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let mut parser = Parser::new();
        parser.set_language(tree_sitter_rust::language())?;
        
        let tree = parser.parse(&content, None).unwrap();
        let root = tree.root_node();
        
        // Extract function definitions
        self.extract_functions(&root, path, symbols);
        // Extract struct definitions
        self.extract_structs(&root, path, symbols);
        
        Ok(())
    }
    
    pub fn find_definition(&self, symbol: &str) -> Option<&Location> {
        self.symbols.get(symbol).and_then(|locs| locs.first())
    }
}
```

### Step 1.2: Dependency Graph
```rust
// context/index/dependency.rs
pub struct DependencyGraph {
    dependencies: HashMap<PathBuf, Vec<PathBuf>>,
}

impl DependencyGraph {
    pub fn build(project_root: &Path) -> Result<Self> {
        let mut dependencies = HashMap::new();
        
        for entry in WalkDir::new(project_root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension() == Some(OsStr::new("rs")))
        {
            let imports = extract_imports(entry.path())?;
            dependencies.insert(entry.path().to_path_buf(), imports);
        }
        
        Ok(DependencyGraph { dependencies })
    }
    
    pub fn get_dependencies(&self, file: &Path) -> Vec<PathBuf> {
        self.dependencies.get(file).cloned().unwrap_or_default()
    }
}

fn extract_imports(file: &Path) -> Result<Vec<PathBuf>> {
    let content = std::fs::read_to_string(file)?;
    let re = Regex::new(r"use\s+(?:crate|super)?::([a-z_][a-z0-9_]*(?:::[a-z_][a-z0-9_]*)*)")?;
    
    // Parse use statements and resolve to file paths
    // This is simplified; real implementation needs proper module resolution
    Ok(vec![])
}
```

### Step 1.3: Minimal Context Builder
```rust
// context/builder.rs
pub struct MinimalContextBuilder {
    symbol_table: Arc<SymbolTable>,
    dep_graph: Arc<DependencyGraph>,
    config: ContextConfig,
}

impl MinimalContextBuilder {
    pub fn build_context(&self, query: &Query) -> Result<Context> {
        // Extract symbols mentioned in query
        let symbols = self.extract_symbols(&query.text);
        
        let mut files = HashSet::new();
        
        // Find definitions
        for symbol in symbols {
            if let Some(location) = self.symbol_table.find_definition(&symbol) {
                files.insert(location.file.clone());
            }
        }
        
        // Add dependencies (up to max_files)
        let mut with_deps = files.clone();
        for file in &files {
            if with_deps.len() >= self.config.max_files {
                break;
            }
            with_deps.extend(self.dep_graph.get_dependencies(file));
        }
        
        // Build file contexts
        let file_contexts: Vec<_> = with_deps.into_iter()
            .take(self.config.max_files)
            .map(|path| FileContext::from_path(&path))
            .collect::<Result<_>>()?;
        
        Ok(Context {
            files: file_contexts,
            system_prompt: SYSTEM_PROMPT.to_string(),
            conversation_history: vec![],
        })
    }
}
```

**Success Criteria**:
- [x] Index builds for Rust projects
- [x] Context size < 30KB average (vs full codebase)
- [x] Relevant files selected for queries
- [x] Daily cost < $7.00 (54% reduction)

**Expected Cost**: $5-7/day
**Savings**: $8-13/day ($240-390/month)
**Time Estimate**: 4 days

---

## Phase 2: Output Optimization (Week 2, Days 1-3)

**Goal**: Reduce output tokens by 35% through constrained generation

**New Features**:
- Concise system prompts
- Diff-based editing
- Response templates
- Token usage tracking

**Implementation Steps**:

### Step 2.1: Optimized System Prompt
```rust
const OPTIMIZED_SYSTEM_PROMPT: &str = r#"
You are a code assistant. Follow these rules:
1. Output ONLY necessary code/changes
2. Use tool calls instead of generating full files
3. Be concise - no explanations unless asked
4. Use line references instead of repeating code

Available tools:
- edit_file(path, start_line, end_line, new_content)
- create_file(path, content)
- search_code(query)
"#;
```

### Step 2.2: Response Parser
```rust
// processing/parser.rs
pub struct ResponseParser;

impl ResponseParser {
    pub fn parse(&self, raw_response: &str) -> Result<ParsedResponse> {
        // Extract tool calls from response
        let tool_calls = self.extract_tool_calls(raw_response)?;
        
        // Calculate confidence based on response quality
        let confidence = self.estimate_confidence(raw_response);
        
        Ok(ParsedResponse {
            text: raw_response.to_string(),
            tool_calls,
            confidence,
        })
    }
    
    fn extract_tool_calls(&self, response: &str) -> Result<Vec<ToolCall>> {
        // Parse structured tool calls from response
        // Look for patterns like: edit_file("src/main.rs", 10, 15, "new content")
        let re = Regex::new(r"edit_file\([^)]+\)")?;
        
        let mut calls = vec![];
        for cap in re.captures_iter(response) {
            // Parse parameters and create ToolCall
            // Simplified for example
        }
        
        Ok(calls)
    }
    
    fn estimate_confidence(&self, response: &str) -> f64 {
        // Heuristic confidence estimation
        if response.contains("I'm not sure") || response.contains("uncertain") {
            0.6
        } else if response.contains("tool_call") || response.contains("edit_file") {
            0.95
        } else {
            0.8
        }
    }
}
```

**Success Criteria**:
- [x] Output tokens < 50K/day (35% reduction)
- [x] Tool calls properly extracted
- [x] Daily cost < $6.00

**Expected Cost**: $4.60-6.00/day
**Savings**: Additional $1-2/day ($30-60/month)
**Time Estimate**: 3 days

---

## Phase 3: Multi-Model Routing (Week 2, Days 4-7)

**Goal**: Route 70% of tasks to cheaper models (Groq/Gemini)

**New Features**:
- Query classifier
- Model router with escalation
- Groq provider
- Gemini provider

**New Modules**:
```
src/routing/
├── classifier.rs        # Query complexity classifier
├── router.rs            # Model tier router
└── escalation.rs        # Escalation logic

src/providers/
├── groq.rs              # Groq API integration
└── gemini.rs            # Gemini Flash integration
```

**Implementation Steps**:

### Step 3.1: Query Classifier
```rust
// routing/classifier.rs
pub struct QueryClassifier;

impl QueryClassifier {
    pub fn classify(&self, query: &Query) -> Complexity {
        let text = query.text.to_lowercase();
        
        // Simple pattern matching
        if self.matches_simple_patterns(&text) {
            Complexity::Simple
        } else if self.matches_complex_patterns(&text) {
            Complexity::Complex
        } else {
            Complexity::Medium
        }
    }
    
    fn matches_simple_patterns(&self, text: &str) -> bool {
        const PATTERNS: &[&str] = &[
            "find", "search", "where", "show", "list", "view"
        ];
        PATTERNS.iter().any(|p| text.contains(p))
    }
    
    fn matches_complex_patterns(&self, text: &str) -> bool {
        const PATTERNS: &[&str] = &[
            "design", "architecture", "refactor all", 
            "migrate", "rewrite", "explain why"
        ];
        PATTERNS.iter().any(|p| text.contains(p))
    }
}
```

### Step 3.2: Model Router
```rust
// routing/router.rs
pub struct ModelRouter {
    providers: HashMap<ProviderTier, Box<dyn ModelProvider>>,
    classifier: QueryClassifier,
}

impl ModelRouter {
    pub async fn route(&self, query: &Query, context: &Context) -> Result<Response> {
        let complexity = self.classifier.classify(query);
        let mut tier = self.initial_tier(complexity);
        
        loop {
            let provider = self.get_provider(tier)?;
            
            match provider.generate(query, context).await {
                Ok(response) if response.confidence >= 0.85 => {
                    return Ok(response);
                }
                Ok(_) if tier < ProviderTier::Premium => {
                    // Escalate to next tier
                    tier = self.escalate(tier)?;
                }
                Ok(response) => {
                    // Already at premium, return anyway
                    return Ok(response);
                }
                Err(e) if e.is_retryable() => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                Err(_) => {
                    tier = self.escalate(tier)?;
                }
            }
        }
    }
    
    fn initial_tier(&self, complexity: Complexity) -> ProviderTier {
        match complexity {
            Complexity::Simple => ProviderTier::FreeTier,  // Groq
            Complexity::Medium => ProviderTier::FreeTier,
            Complexity::Complex => ProviderTier::Budget,   // Gemini
        }
    }
}
```

### Step 3.3: Groq Provider
```rust
// providers/groq.rs
pub struct GroqProvider {
    client: reqwest::Client,
    api_key: String,
}

#[async_trait]
impl ModelProvider for GroqProvider {
    fn name(&self) -> &str { "groq" }
    
    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let request = json!({
            "model": "llama-3.1-70b-versatile",
            "messages": [
                {"role": "system", "content": context.system_prompt},
                {"role": "user", "content": format!("{}\n\nContext:\n{}", query.text, context.files_to_string())}
            ],
            "temperature": 0.2,
        });
        
        let resp = self.client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await?;
        
        let result: serde_json::Value = resp.json().await?;
        
        Ok(Response {
            text: result["choices"][0]["message"]["content"].as_str().unwrap().to_string(),
            confidence: 0.85, // Groq doesn't provide confidence
            tool_calls: vec![],
            tokens_used: TokenUsage::from_groq_response(&result),
            provider: "groq".to_string(),
            latency_ms: 0,
        })
    }
    
    fn tier(&self) -> ProviderTier { ProviderTier::FreeTier }
}
```

**Success Criteria**:
- [x] 60%+ tasks routed to Groq/Gemini
- [x] Escalation rate < 20%
- [x] Daily cost < $3.50 (77% reduction)

**Expected Cost**: $2.20-3.50/day
**Savings**: Additional $2-4/day ($60-120/month)
**Time Estimate**: 4 days

---

## Phase 4: Local Model Integration (Week 3)

**Goal**: Handle 70% of tasks locally for $0 cost

**New Features**:
- Ollama integration
- Local model provider (Phi-3, Qwen2.5-Coder)
- Fast local routing
- Confidence-based escalation

**New Modules**:
```
src/providers/
└── local.rs             # Ollama integration

src/routing/
└── local_router.rs      # Fast local classification
```

**Implementation Steps**:

### Step 4.1: Ollama Provider
```rust
// providers/local.rs
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: "http://localhost:11434".to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait]
impl ModelProvider for OllamaProvider {
    fn name(&self) -> &str { "ollama" }
    
    async fn is_available(&self) -> bool {
        self.client.get(&format!("{}/api/tags", self.base_url))
            .send()
            .await
            .is_ok()
    }
    
    async fn generate(&self, query: &Query, context: &Context) -> Result<Response> {
        let start = Instant::now();
        
        let request = json!({
            "model": self.model,
            "prompt": format!("{}\n\n{}", context.system_prompt, query.text),
            "stream": false,
            "options": {
                "temperature": 0.2,
            }
        });
        
        let resp = self.client
            .post(&format!("{}/api/generate", self.base_url))
            .json(&request)
            .send()
            .await?;
        
        let result: serde_json::Value = resp.json().await?;
        let text = result["response"].as_str().unwrap().to_string();
        
        Ok(Response {
            text,
            confidence: self.estimate_confidence(&text),
            tool_calls: vec![],
            tokens_used: TokenUsage::default(), // Ollama doesn't track cache
            provider: format!("ollama-{}", self.model),
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
    
    fn estimate_cost(&self, _context: &Context) -> f64 { 0.0 }
    fn tier(&self) -> ProviderTier { ProviderTier::Local }
}
```

### Step 4.2: Two-Tier Local Setup
```rust
pub struct LocalModelManager {
    router: OllamaProvider,  // phi3:mini for fast routing
    coder: OllamaProvider,   // qwen2.5-coder:7b for code tasks
}

impl LocalModelManager {
    pub fn new() -> Self {
        Self {
            router: OllamaProvider::new("phi3:mini"),
            coder: OllamaProvider::new("qwen2.5-coder:7b"),
        }
    }
    
    pub async fn handle_locally(&self, query: &Query) -> Option<Response> {
        // Step 1: Quick classification with router model
        let classification = self.router.classify_quickly(query).await.ok()?;
        
        match classification {
            LocalTask::SimpleSearch => {
                // Handle with local index, no model needed
                Some(self.search_locally(query))
            }
            LocalTask::CodeEdit => {
                // Use coder model
                self.coder.generate(query, &Context::minimal()).await.ok()
            }
            LocalTask::TooComplex => {
                // Escalate to cloud
                None
            }
        }
    }
}
```

**Success Criteria**:
- [x] Ollama integration works
- [x] 70%+ tasks handled locally
- [x] Local inference < 500ms P95
- [x] Daily cost < $1.50

**Expected Cost**: $0.50-1.50/day
**Savings**: Additional $1-3/day ($30-90/month)
**Time Estimate**: 7 days

---

## Phase 5: Advanced Optimizations (Week 4)

**Goal**: Fine-tune for maximum efficiency and quality

**New Features**:
- Semantic search with embeddings (optional)
- Conversation-aware caching
- Advanced metrics and analytics
- A/B testing framework

**Implementation Steps**:

### Step 5.1: Semantic Search (Optional)
```rust
// context/index/embeddings.rs
use fastembed::{EmbeddingModel, InitOptions};

pub struct SemanticIndex {
    embeddings: Vec<(PathBuf, Vec<f32>)>,
    model: EmbeddingModel,
}

impl SemanticIndex {
    pub fn build(project_root: &Path) -> Result<Self> {
        let model = EmbeddingModel::try_new(InitOptions::default())?;
        let mut embeddings = vec![];
        
        for file in list_code_files(project_root) {
            let content = std::fs::read_to_string(&file)?;
            let embedding = model.embed_one(&content)?;
            embeddings.push((file, embedding));
        }
        
        Ok(Self { embeddings, model })
    }
    
    pub fn search(&self, query: &str, top_k: usize) -> Vec<PathBuf> {
        let query_embedding = self.model.embed_one(query).unwrap();
        
        let mut scores: Vec<_> = self.embeddings.iter()
            .map(|(path, emb)| {
                let similarity = cosine_similarity(&query_embedding, emb);
                (path.clone(), similarity)
            })
            .collect();
        
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scores.into_iter().take(top_k).map(|(p, _)| p).collect()
    }
}
```

### Step 5.2: Advanced Metrics
```rust
// observability/metrics.rs
pub struct AdvancedMetrics {
    base: MetricsCollector,
    
    // Quality tracking
    user_feedback: DashMap<String, f64>,
    
    // A/B testing
    variant_metrics: DashMap<String, VariantMetrics>,
}

impl AdvancedMetrics {
    pub fn record_feedback(&self, request_id: &str, rating: f64) {
        self.user_feedback.insert(request_id.to_string(), rating);
    }
    
    pub fn compare_variants(&self, variant_a: &str, variant_b: &str) -> Comparison {
        let a_metrics = self.variant_metrics.get(variant_a).unwrap();
        let b_metrics = self.variant_metrics.get(variant_b).unwrap();
        
        Comparison {
            cost_diff: a_metrics.avg_cost - b_metrics.avg_cost,
            quality_diff: a_metrics.avg_quality - b_metrics.avg_quality,
            speed_diff: a_metrics.avg_latency - b_metrics.avg_latency,
        }
    }
}
```

**Success Criteria**:
- [x] All optimizations working together
- [x] Daily cost < $0.60 (96% reduction)
- [x] Success rate > 90%
- [x] P95 latency < 3s

**Expected Cost**: $0.15-0.60/day
**Savings**: Additional $0.50-1.00/day ($15-30/month)
**Time Estimate**: 7 days

---

## Testing Strategy

### Per-Phase Testing

**MVP Testing**:
```rust
#[tokio::test]
async fn test_basic_query() {
    let provider = AnthropicProvider::new_from_env();
    let query = Query::new("Find main function");
    let context = Context::minimal();
    
    let response = provider.generate(&query, &context).await.unwrap();
    assert!(!response.text.is_empty());
}
```

**Context Optimization Testing**:
```rust
#[test]
fn test_symbol_extraction() {
    let builder = MinimalContextBuilder::new();
    let query = Query::new("Refactor the parse_response function");
    
    let symbols = builder.extract_symbols(&query.text);
    assert!(symbols.contains(&"parse_response".to_string()));
}

#[test]
fn test_context_size_limit() {
    let builder = MinimalContextBuilder::with_max_files(5);
    let context = builder.build_context(&query).unwrap();
    
    assert!(context.files.len() <= 5);
}
```

**Routing Testing**:
```rust
#[test]
fn test_complexity_classification() {
    let classifier = QueryClassifier::new();
    
    assert_eq!(
        classifier.classify(&Query::new("find function foo")),
        Complexity::Simple
    );
    
    assert_eq!(
        classifier.classify(&Query::new("design a new authentication system")),
        Complexity::Complex
    );
}

#[tokio::test]
async fn test_escalation() {
    let mut router = ModelRouter::new();
    router.add_provider(ProviderTier::Local, MockProvider::low_confidence());
    router.add_provider(ProviderTier::Budget, MockProvider::high_confidence());
    
    let response = router.route(&query, &context).await.unwrap();
    assert_eq!(response.provider, "budget"); // Should have escalated
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_full_flow() {
    let app = AgenticOptimizer::new_with_test_config();
    
    let response = app.process_query("Refactor main.rs").await.unwrap();
    
    assert!(response.confidence > 0.8);
    assert!(!response.tool_calls.is_empty());
}
```

### Benchmark Testing

```rust
#[bench]
fn bench_context_building(b: &mut Bencher) {
    let builder = MinimalContextBuilder::new();
    let query = Query::new("test");
    
    b.iter(|| builder.build_context(&query));
}

#[bench]
fn bench_local_inference(b: &mut Bencher) {
    let provider = OllamaProvider::new("phi3:mini");
    let query = Query::new("classify this");
    
    b.iter(|| {
        block_on(provider.generate(&query, &Context::minimal()))
    });
}
```

---

## Cost Tracking Per Phase

| Phase | Daily Cost | Monthly Cost | Savings | Cumulative Savings |
|-------|-----------|--------------|---------|-------------------|
| **Current** | $15.20 | $456 | - | - |
| Phase 0 (MVP) | $15.20 | $456 | $0 | $0 |
| Phase 1 (Context) | $5.20 | $156 | $300 | $300 |
| Phase 2 (Output) | $4.60 | $138 | $18 | $318 |
| Phase 3 (Multi-model) | $2.50 | $75 | $63 | $381 |
| Phase 4 (Local) | $0.80 | $24 | $51 | $432 |
| Phase 5 (Advanced) | $0.50 | $15 | $9 | $441 |

**Final Target**: $0.50/day, **96.7% reduction**, **$441/month savings**

---

## Rollout Strategy

### Feature Flags

Use feature flags to control optimizations:

```toml
[features]
context-optimization = true
output-optimization = true
multi-model-routing = true
local-models = true
semantic-search = false  # Optional, experimental
```

### Gradual Rollout

1. **Week 1**: MVP → Phase 1 (Context optimization)
2. **Week 2**: Phase 2 (Output) → Phase 3 (Multi-model)
3. **Week 3**: Phase 4 (Local models)
4. **Week 4**: Phase 5 (Advanced features)

### Monitoring

Track metrics daily:
```bash
cargo run -- metrics --daily
```

Expected output:
```
Daily Metrics (2025-10-15):
  Total Requests: 45
  Success Rate: 94.4%
  
  Costs:
    Local:  $0.00 (32 requests, 71%)
    Groq:   $0.00 (7 requests, 15%)
    Gemini: $0.25 (4 requests, 9%)
    Sonnet: $0.30 (2 requests, 4%)
    Total:  $0.55
  
  Performance:
    P50 latency: 180ms
    P95 latency: 2.1s
    P99 latency: 4.5s
```

---

## Next Steps

1. **Review this plan** with team/stakeholders
2. **Set up development environment** (Rust, Ollama, API keys)
3. **Create GitHub issues** for each phase
4. **Start Phase 0 implementation** (MVP)
5. **Ship early, iterate fast**

**Timeline**: 4-5 weeks to full optimization
**Risk**: Low (incremental, testable phases)
**ROI**: $441/month savings ($5,292/year)
