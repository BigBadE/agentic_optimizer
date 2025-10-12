# Merlin: Next-Generation AI Coding Agent
## Comprehensive Development Roadmap

**Vision**: Build the most capable, efficient, and adaptable AI coding agent for managing large complex codebases

**Current Status**: Phase 5 Complete | 160 Rust files | 264 tests passing
**Next Phase**: 6 - Agent Intelligence & Tool Innovation

---

## Table of Contents

1. [Current Architecture](#current-architecture)
2. [Core Design Philosophy](#core-design-philosophy)
3. [Phase 6: Agent Intelligence & Tool Innovation](#phase-6-agent-intelligence--tool-innovation)
4. [Phase 7: Context Mastery](#phase-7-context-mastery)
5. [Phase 8: Multi-Model Orchestra](#phase-8-multi-model-orchestra)
6. [Phase 9: Performance & Scale](#phase-9-performance--scale)
7. [Phase 10: Adaptive Intelligence](#phase-10-adaptive-intelligence)
8. [Success Metrics](#success-metrics)
9. [Risk Mitigation](#risk-mitigation)

---

## Current Architecture

### What Works Today (Phases 0-5 ✅)

**9-Crate Workspace**:
- `merlin-core` - Fundamental types, traits, error handling
- `merlin-context` - Context building with semantic search (BM25 + embeddings)
- `merlin-routing` - Multi-tier model routing, task execution, validation, TUI
- `merlin-providers` - External APIs (Groq, OpenRouter, Anthropic, DeepSeek)
- `merlin-local` - Ollama integration for local models
- `merlin-agent` - Agent execution, self-assessment, streaming
- `merlin-languages` - Language backends (rust-analyzer integration)
- `merlin-tools` - File operations, command execution
- `merlin-cli` - Command-line interface and configuration

**Model Routing Pipeline**:
```
Query → Analyzer → Router → Executor → Validator → Result
  ↓         ↓         ↓         ↓          ↓
Local    Complexity  Tier     Tools    Syntax
(Qwen)   Assessment  Select   +Files   Build
  ↓         ↓         ↓         ↓       Test
Groq     Intent      Fallback Streaming Lint
(Llama)  Detection   Chain    Progress
  ↓         ↓         ↓         ↓
Premium  Subtasks   Escalate  Cache
(Claude) Parallel   Retry     Metrics
```

**Key Features**:
- ✅ Query intent classification (Conversational vs CodeQuery vs CodeModification)
- ✅ Conversation history with 50-message limit
- ✅ Multi-stage validation pipeline
- ✅ Task decomposition with 4 execution strategies
- ✅ Self-determining tasks with automatic assessment
- ✅ Response caching with semantic similarity (0.95 threshold)
- ✅ TUI with real-time progress, task trees, streaming output
- ✅ File locking and conflict detection
- ✅ Workspace isolation with snapshots
- ✅ Cost tracking and metrics collection
- ✅ TOML configuration support

### Current Performance

**Speed**:
- Local (Qwen 7B): ~100-500ms
- Groq (Llama 70B): ~500-2000ms
- Premium (Claude/DeepSeek): ~2000-5000ms

**Cost** (estimated daily):
- Local: $0 (unlimited)
- Groq: $0 (free tier: 14,400 requests/day)
- Premium: $0.50-$2.00 (depending on usage)

**Quality**:
- Test Coverage: ~35% (264 tests)
- Success Rate: ~85-90% (estimated, needs tracking)
- Escalation Rate: ~20-25% (needs reduction)

---

## Core Design Philosophy

### Guiding Principles

1. **Context is King**: The best model with wrong context fails; a weaker model with perfect context succeeds
2. **Parallel by Default**: Maximize throughput with safe concurrent execution
3. **Fail Fast, Learn Faster**: Quick validation cycles with immediate feedback
4. **Cost-Conscious Intelligence**: Use the cheapest model that can solve the problem
5. **Language-Aware**: Deep integration with language-specific tooling (LSP, analyzers)
6. **Human-Centric UX**: Real-time feedback, clear progress, easy debugging

### Inspiration from Best Agents

**Claude Code (claude.ai/code)**:
- ✅ Streaming output with real-time progress
- ✅ Tool use with structured JSON
- ❌ No local model support
- ❌ Single-threaded execution

**Cursor**:
- ✅ Fast inline completions
- ✅ Codebase-wide understanding
- ❌ Proprietary, closed-source
- ❌ Limited to IDE

**Aider**:
- ✅ Git integration
- ✅ Multi-file editing
- ❌ Limited context window management
- ❌ No parallel execution

**Devin**:
- ✅ Long-running autonomous tasks
- ✅ Browser and terminal control
- ❌ Expensive ($500/month)
- ❌ Slow iteration cycles

**Our Differentiators**:
1. **TypeScript Tool Syntax** (Phase 6.1) - Natural for LLMs trained on open-source code
2. **Adaptive Context Windows** (Phase 7.2) - Dynamic sizing based on task complexity
3. **Model Specialization** (Phase 8.2) - Right model for each subtask (code gen, review, test)
4. **Parallel Tool Execution** (Phase 6.3) - 5-10x faster on multi-file operations
5. **Language-Specific Backends** (Phase 7.5) - LSP integration for precise navigation

---

## Phase 6: Agent Intelligence & Tool Innovation
**Timeline**: 6-8 weeks
**Priority**: Critical (Foundation for all future work)

### 6.1 TypeScript Tool Syntax (Revolutionary) 🔥 [IN PROGRESS]

**Status**: ✅ Basic implementation complete (simplified parser)
**Next**: Full SWC/Deno integration for complete TypeScript support

**Completed**:
- ✅ Basic TypeScript runtime with simplified parser
- ✅ Support for `await functionName(arg1, arg2)` syntax
- ✅ String, number, boolean, null literal parsing
- ✅ Multiple tool calls in sequence
- ✅ Tool validation and error handling
- ✅ Type definition generation for LLM context
- ✅ Comprehensive tests (22 tests passing)
- ✅ Documentation in README.md
- ✅ Zero clippy warnings with strict linting

**Next Steps (Future Work)**:
- ⏭️ Full SWC parser integration (blocked by nightly/cranelift compatibility)
- ⏭️ Deno runtime for complete JavaScript execution
- ⏭️ Control flow support (loops, conditionals, variables)
- ⏭️ Complex expressions and operators
- ⏭️ Try/catch error handling

**Note**: Full SWC/Deno integration encountered compatibility issues with the nightly Rust toolchain and cranelift backend. The basic implementation provides immediate value for simple tool call patterns, which covers 80% of use cases. Full TypeScript support can be added later when toolchain compatibility improves or by using a different runtime approach (e.g., QuickJS, Boa).

**Problem**: Current tool JSON syntax is unnatural for LLMs
```json
{
  "name": "read_file",
  "parameters": {
    "path": "src/main.rs"
  }
}
```

**Solution**: TypeScript function calls (LLMs are trained on this!)
```typescript
// Agent generates this naturally:
await readFile("src/main.rs")
await writeFile("src/lib.rs", content)
await runCommand("cargo", ["test", "--", "--nocapture"])

// Multi-step with clear intent:
const tests = await listFiles("tests/**/*.rs")
for (const test of tests) {
  const content = await readFile(test)
  if (content.includes("TODO")) {
    await writeFile(test, content.replace("TODO", "FIXED"))
  }
}
```

**Why This Works**:
1. LLMs see millions of examples in training data
2. Natural control flow (loops, conditions, error handling)
3. Type hints guide correct parameter usage
4. Reduces hallucination (familiar syntax)
5. Enables tool chaining without special syntax

**Implementation**:
```rust
// crates/merlin-tools/src/typescript_runtime.rs
pub struct TypeScriptToolRuntime {
    executor: JsRuntime,  // Using deno_core
    tool_registry: Arc<ToolRegistry>,
}

impl TypeScriptToolRuntime {
    pub async fn execute(&mut self, code: &str) -> Result<Value> {
        // 1. Parse TypeScript to AST
        let ast = self.parse_typescript(code)?;

        // 2. Extract tool calls
        let calls = self.extract_tool_calls(&ast)?;

        // 3. Execute tools with proper awaits
        for call in calls {
            match call.name.as_str() {
                "readFile" => self.handle_read_file(call.args).await?,
                "writeFile" => self.handle_write_file(call.args).await?,
                "runCommand" => self.handle_run_command(call.args).await?,
                _ => return Err(ToolError::UnknownTool(call.name)),
            }
        }

        Ok(Value::Null)
    }
}
```

**System Prompt Addition**:
```
You have access to these TypeScript functions:

async function readFile(path: string): Promise<string>
async function writeFile(path: string, content: string): Promise<void>
async function listFiles(glob: string): Promise<string[]>
async function runCommand(cmd: string, args: string[]): Promise<{stdout: string, stderr: string, code: number}>
async function searchCode(pattern: string, files?: string[]): Promise<SearchResult[]>

Write TypeScript code to accomplish the task. The code will be executed in a sandboxed environment.
```

**Safety**:
- Sandboxed execution (no network, limited filesystem)
- Timeout limits (30s per script)
- Resource limits (memory, CPU)
- Validation before execution

**Testing**:
- 50+ tests for common patterns
- Fuzzing for malicious inputs
- Performance benchmarks

**Files**:
- `crates/merlin-tools/src/typescript_runtime.rs` (800 lines)
- `crates/merlin-tools/src/typescript_parser.rs` (400 lines)
- `crates/merlin-tools/src/typescript_validator.rs` (300 lines)
- `crates/merlin-tools/tests/typescript_integration_tests.rs` (500 lines)

### 6.2 Smart Tool Chaining

**Problem**: Tools execute sequentially, even when parallel is safe

**Current**:
```
read src/main.rs → read src/lib.rs → read src/utils.rs  (sequential, 300ms)
```

**Solution**: Dependency analysis + parallel execution
```
read [src/main.rs, src/lib.rs, src/utils.rs]  (parallel, 100ms)
```

**Implementation**:
```rust
pub struct ToolChain {
    steps: Vec<ToolStep>,
    dependencies: HashMap<StepId, Vec<StepId>>,
}

impl ToolChain {
    pub fn analyze_dependencies(&self) -> ExecutionGraph {
        // Build DAG of dependencies
        let mut graph = Graph::new();
        for step in &self.steps {
            graph.add_node(step.id);
            for dep in self.find_dependencies(step) {
                graph.add_edge(dep.id, step.id);
            }
        }
        graph
    }

    pub async fn execute_parallel(&self) -> Result<Vec<ToolResult>> {
        let graph = self.analyze_dependencies();
        let batches = graph.topological_batches();

        let mut results = Vec::new();
        for batch in batches {
            // Execute all tools in batch concurrently
            let batch_results = join_all(
                batch.iter().map(|step| self.execute_step(step))
            ).await;
            results.extend(batch_results);
        }
        Ok(results)
    }
}
```

**Dependency Detection**:
- **Read → Write**: Must be sequential if same file
- **Read → Read**: Always parallel
- **Write → Write**: Sequential if same file
- **Command → Command**: Depends on working directory

**Files**:
- `crates/merlin-tools/src/chain.rs` (600 lines)
- `crates/merlin-tools/src/dependency_analysis.rs` (400 lines)

### 6.3 Documentation Tracking

**Problem**: Code changes, docs fall behind

**Solution**: Automatic documentation maintenance
```rust
pub struct DocTracker {
    codebase_state: CodebaseSnapshot,
    doc_locations: HashMap<Symbol, Vec<DocLocation>>,
}

impl DocTracker {
    pub async fn check_staleness(&self) -> Vec<StaleDoc> {
        let mut stale = Vec::new();

        for (symbol, locs) in &self.doc_locations {
            let current_sig = self.get_signature(symbol);
            for loc in locs {
                let doc_sig = self.extract_documented_signature(loc);
                if doc_sig != current_sig {
                    stale.push(StaleDoc {
                        location: loc.clone(),
                        symbol: symbol.clone(),
                        reason: SignatureMismatch {
                            expected: current_sig.clone(),
                            documented: doc_sig,
                        },
                    });
                }
            }
        }

        stale
    }

    pub async fn suggest_updates(&self, stale: &[StaleDoc]) -> Vec<DocUpdate> {
        // Use LLM to generate doc updates
        let mut updates = Vec::new();
        for doc in stale {
            let context = self.build_doc_context(doc);
            let suggestion = self.llm.generate_doc_update(&context).await?;
            updates.push(DocUpdate {
                location: doc.location.clone(),
                old_content: doc.current_content.clone(),
                new_content: suggestion,
            });
        }
        updates
    }
}
```

**Integration**:
- Pre-commit hook: Check for stale docs
- Post-refactor: Suggest doc updates
- CI: Fail if critical docs are stale

**Files**:
- `crates/merlin-context/src/doc_tracker.rs` (500 lines)
- `crates/merlin-context/src/signature_matching.rs` (300 lines)

### 6.4 Coding Standards Enforcement

**Problem**: Inconsistent style, patterns, conventions

**Solution**: Learn from codebase, enforce automatically
```rust
pub struct StyleGuide {
    patterns: Vec<CodePattern>,
    violations: Vec<Violation>,
}

impl StyleGuide {
    pub async fn learn_from_codebase(&mut self, files: &[PathBuf]) -> Result<()> {
        // Extract patterns from existing code
        for file in files {
            let ast = parse_file(file)?;
            self.patterns.extend(self.extract_patterns(&ast));
        }

        // Find common patterns
        self.patterns = self.find_consensus_patterns();
        Ok(())
    }

    pub fn check_code(&self, code: &str) -> Vec<Violation> {
        let mut violations = Vec::new();
        let ast = parse_code(code).unwrap();

        for pattern in &self.patterns {
            if let Some(violation) = pattern.check(&ast) {
                violations.push(violation);
            }
        }

        violations
    }

    pub fn suggest_fixes(&self, violations: &[Violation]) -> Vec<Fix> {
        violations.iter()
            .filter_map(|v| v.suggested_fix())
            .collect()
    }
}
```

**Patterns Tracked**:
- Naming conventions (snake_case, PascalCase, SCREAMING_SNAKE)
- Error handling style (Result vs panic)
- Import organization
- Comment style (doc comments, inline, TODO format)
- Function length limits
- Complexity limits

**Files**:
- `crates/merlin-languages/src/style_guide.rs` (700 lines)
- `crates/merlin-languages/src/pattern_extraction.rs` (500 lines)

---

## Phase 7: Context Mastery
**Timeline**: 6-8 weeks
**Priority**: High (Directly improves quality)

### 7.1 Intelligent Context Pruning

**Problem**: Too many irrelevant files in context (current: 18 files for "remember bacon")

**Solution**: Multi-stage relevance filtering
```rust
pub struct ContextPruner {
    bm25: BM25Scorer,
    embeddings: EmbeddingModel,
    graph: DependencyGraph,
}

impl ContextPruner {
    pub async fn select_optimal_files(
        &self,
        query: &Query,
        candidates: Vec<FileScore>,
        max_tokens: usize,
    ) -> Vec<FileContext> {
        // Stage 1: Keyword matching (fast)
        let keyword_filtered = self.bm25.top_k(&candidates, 50);

        // Stage 2: Semantic similarity (medium)
        let semantic_filtered = self.embeddings
            .rank_by_similarity(query, &keyword_filtered)
            .take(20);

        // Stage 3: Dependency analysis (precise)
        let with_deps = self.graph
            .expand_with_dependencies(&semantic_filtered);

        // Stage 4: Token budget optimization
        self.optimize_token_budget(with_deps, max_tokens)
    }

    fn optimize_token_budget(
        &self,
        files: Vec<FileContext>,
        max_tokens: usize,
    ) -> Vec<FileContext> {
        // Knapsack problem: maximize relevance within token budget
        let mut dp = vec![vec![0.0; max_tokens + 1]; files.len() + 1];

        for i in 1..=files.len() {
            let file = &files[i - 1];
            let tokens = file.token_count();
            let relevance = file.relevance_score;

            for t in 0..=max_tokens {
                dp[i][t] = dp[i - 1][t];  // Don't include
                if tokens <= t {
                    dp[i][t] = dp[i][t].max(
                        dp[i - 1][t - tokens] + relevance
                    );
                }
            }
        }

        // Backtrack to find selected files
        self.backtrack_selection(&dp, &files, max_tokens)
    }
}
```

**Heuristics**:
- Recently modified files: +20% relevance
- Files imported by query-mentioned files: +30%
- Test files for implementation files: +15%
- README/docs for library questions: +25%

**Files**:
- `crates/merlin-context/src/pruning.rs` (600 lines)
- `crates/merlin-context/src/knapsack_optimizer.rs` (300 lines)

### 7.2 Adaptive Context Windows

**Problem**: Fixed token limits waste capacity on simple tasks, overflow on complex ones

**Solution**: Dynamic window sizing based on task complexity
```rust
pub struct AdaptiveContextWindow {
    base_budget: usize,      // 4000 tokens
    max_budget: usize,       // 100000 tokens
    complexity_estimator: ComplexityEstimator,
}

impl AdaptiveContextWindow {
    pub fn calculate_budget(&self, task: &Task) -> usize {
        let complexity = self.complexity_estimator.estimate(task);

        match complexity {
            Complexity::Trivial => 2_000,     // Greeting, simple query
            Complexity::Simple => 8_000,      // Single-file edit
            Complexity::Moderate => 32_000,   // Multi-file refactor
            Complexity::Complex => 100_000,   // Architecture change
        }
    }

    pub fn should_expand(&self, current: &Context, task: &Task) -> bool {
        // Expand if we're missing critical information
        let missing_symbols = self.find_missing_symbols(current, task);
        let missing_deps = self.find_missing_dependencies(current);

        !missing_symbols.is_empty() || !missing_deps.is_empty()
    }
}
```

**Budget Allocation**:
- System prompt: 10-15%
- Conversation history: 5-10%
- Code context: 60-75%
- Tool descriptions: 5-10%
- Reserved for response: 10-15%

**Files**:
- `crates/merlin-context/src/adaptive_window.rs` (400 lines)

### 7.3 Conversation Summarization

**Problem**: 50-message limit discards useful context

**Solution**: Hierarchical summarization
```rust
pub struct ConversationSummarizer {
    llm: Arc<dyn ModelProvider>,
    summary_cache: HashMap<MessageRange, String>,
}

impl ConversationSummarizer {
    pub async fn summarize_old_messages(
        &mut self,
        messages: &[(String, String)],
        keep_recent: usize,
    ) -> ConversationWithSummary {
        let (old, recent) = messages.split_at(messages.len() - keep_recent);

        // Summarize in chunks of 10 messages
        let mut summaries = Vec::new();
        for chunk in old.chunks(10) {
            let summary = self.summarize_chunk(chunk).await?;
            summaries.push(summary);
        }

        // Recursively summarize summaries if needed
        let final_summary = if summaries.len() > 5 {
            self.summarize_summaries(&summaries).await?
        } else {
            summaries.join("\n\n")
        };

        ConversationWithSummary {
            summary: final_summary,
            recent_messages: recent.to_vec(),
        }
    }
}
```

**Summary Format**:
```
=== Conversation Summary (Messages 1-40) ===
User requested implementation of authentication system.
Agent created User model, login endpoint, JWT middleware.
User reported bug with token expiration.
Agent fixed expiration logic in jwt.rs.
=== End Summary ===

=== Recent Messages (41-50) ===
[Full message history]
```

**Files**:
- `crates/merlin-context/src/conversation_summarizer.rs` (500 lines)

### 7.4 Semantic Code Search

**Problem**: Keyword search misses semantically similar code

**Solution**: Vector embeddings for code search
```rust
pub struct SemanticCodeSearch {
    embeddings: EmbeddingStore,
    index: HNSW,  // Hierarchical Navigable Small World graph
}

impl SemanticCodeSearch {
    pub async fn search(
        &self,
        query: &str,
        top_k: usize,
    ) -> Vec<CodeChunk> {
        // Embed query
        let query_vec = self.embeddings.embed(query).await?;

        // Search in HNSW index (sub-millisecond)
        let candidates = self.index.search(&query_vec, top_k * 2);

        // Re-rank with cross-encoder for precision
        let reranked = self.rerank(query, &candidates);

        reranked.into_iter().take(top_k).collect()
    }

    pub async fn index_codebase(&mut self, files: &[PathBuf]) -> Result<()> {
        for file in files {
            let chunks = self.chunk_file(file).await?;
            for chunk in chunks {
                let embedding = self.embeddings.embed(&chunk.text).await?;
                self.index.insert(chunk.id, embedding);
            }
        }
        Ok(())
    }
}
```

**Chunking Strategy**:
- Functions: Complete function body
- Structs: Struct definition + impl blocks
- Modules: Module-level docs + exports
- Tests: Complete test function

**Files**:
- `crates/merlin-context/src/semantic_search.rs` (600 lines)
- `crates/merlin-context/src/hnsw_index.rs` (400 lines)

### 7.5 Language-Specific Navigation

**Problem**: Generic file search doesn't understand code structure

**Solution**: LSP integration for precise navigation
```rust
pub struct RustNavigator {
    analyzer: rust_analyzer::Analysis,
}

impl LanguageNavigator for RustNavigator {
    async fn find_definition(&self, position: Position) -> Vec<Location> {
        self.analyzer.goto_definition(position).await
    }

    async fn find_references(&self, symbol: &Symbol) -> Vec<Reference> {
        self.analyzer.find_all_references(symbol).await
    }

    async fn find_implementations(&self, trait_name: &str) -> Vec<Impl> {
        self.analyzer.goto_implementation(trait_name).await
    }

    async fn get_call_hierarchy(&self, function: &str) -> CallGraph {
        self.analyzer.call_hierarchy(function).await
    }
}
```

**Use Cases**:
- "Find all calls to this function" → Use LSP, not grep
- "Show implementations of this trait" → LSP knows structure
- "Where is this type defined?" → LSP gives exact location

**Files**:
- `crates/merlin-languages/src/rust/navigator.rs` (500 lines)
- `crates/merlin-languages/src/navigator_trait.rs` (200 lines)

---

## Phase 8: Multi-Model Orchestra
**Timeline**: 4-6 weeks
**Priority**: High (Cost reduction + quality improvement)

### 8.1 Task-Specific Model Routing

**Problem**: Using same model for all subtasks is suboptimal

**Current**:
```
Claude Sonnet (all tasks) → $0.50 per request
```

**Solution**: Specialize models by task type
```
Code generation → DeepSeek Coder ($0.01)
Code review → Claude Sonnet ($0.15)
Test generation → Qwen 7B ($0)
Documentation → Groq Llama 70B ($0)
Architecture → Claude Opus ($0.50)
```

**Implementation**:
```rust
pub struct TaskSpecificRouter {
    models: HashMap<TaskCategory, Vec<ModelSpec>>,
}

impl TaskSpecificRouter {
    pub fn select_model(&self, task: &Task) -> ModelSpec {
        let category = self.categorize_task(task);
        let candidates = &self.models[&category];

        // Select based on complexity within category
        match task.complexity {
            Complexity::Simple => candidates[0].clone(),      // Cheapest
            Complexity::Moderate => candidates[1].clone(),    // Balanced
            Complexity::Complex => candidates[2].clone(),     // Best
        }
    }

    fn categorize_task(&self, task: &Task) -> TaskCategory {
        let desc = task.description.to_lowercase();

        if desc.contains("implement") || desc.contains("write") {
            TaskCategory::CodeGeneration
        } else if desc.contains("review") || desc.contains("check") {
            TaskCategory::CodeReview
        } else if desc.contains("test") {
            TaskCategory::TestGeneration
        } else if desc.contains("document") {
            TaskCategory::Documentation
        } else if desc.contains("architecture") || desc.contains("design") {
            TaskCategory::Architecture
        } else {
            TaskCategory::General
        }
    }
}
```

**Cost Savings**:
- Current average: $0.15/request
- With specialization: $0.03/request
- **5x cost reduction**

**Files**:
- `crates/merlin-routing/src/router/task_specific.rs` (500 lines)

### 8.2 Ensemble Validation

**Problem**: Single model can make subtle mistakes

**Solution**: Multiple models vote on correctness
```rust
pub struct EnsembleValidator {
    reviewers: Vec<Arc<dyn ModelProvider>>,
    consensus_threshold: f32,  // 0.7 = 70% agreement
}

impl EnsembleValidator {
    pub async fn validate(&self, code: &str, requirements: &str) -> ValidationResult {
        // Parallel reviews from multiple models
        let reviews = join_all(
            self.reviewers.iter().map(|model| {
                self.get_review(model, code, requirements)
            })
        ).await;

        // Aggregate scores
        let avg_score = reviews.iter().map(|r| r.score).sum::<f32>() / reviews.len() as f32;
        let agreement = self.calculate_agreement(&reviews);

        if agreement >= self.consensus_threshold {
            ValidationResult::Pass {
                confidence: agreement,
                notes: self.merge_feedback(&reviews),
            }
        } else {
            ValidationResult::Uncertain {
                reviews,
                requires_human: true,
            }
        }
    }
}
```

**Reviewers**:
- Fast pass: Qwen 7B (local, $0)
- Standard: Groq Llama 70B ($0)
- Critical: Claude Sonnet ($0.15)

**Files**:
- `crates/merlin-routing/src/validator/ensemble.rs` (400 lines)

### 8.3 Model Capability Profiles

**Problem**: Don't know which models are good at what

**Solution**: Benchmark and track model capabilities
```rust
pub struct ModelCapabilityProfile {
    model: ModelSpec,
    capabilities: HashMap<Capability, Score>,
}

pub struct CapabilityBenchmark {
    profiles: HashMap<String, ModelCapabilityProfile>,
}

impl CapabilityBenchmark {
    pub async fn benchmark_model(&mut self, model: &ModelSpec) -> ModelCapabilityProfile {
        let mut capabilities = HashMap::new();

        // Test code generation
        capabilities.insert(
            Capability::CodeGeneration,
            self.test_code_generation(model).await,
        );

        // Test refactoring
        capabilities.insert(
            Capability::Refactoring,
            self.test_refactoring(model).await,
        );

        // Test test generation
        capabilities.insert(
            Capability::TestGeneration,
            self.test_test_generation(model).await,
        );

        ModelCapabilityProfile {
            model: model.clone(),
            capabilities,
        }
    }

    pub fn select_best_for_capability(&self, cap: Capability) -> &ModelSpec {
        self.profiles.values()
            .max_by_key(|p| p.capabilities.get(&cap).unwrap())
            .map(|p| &p.model)
            .unwrap()
    }
}
```

**Capabilities Tracked**:
- Code generation (correctness, style, efficiency)
- Refactoring (safety, completeness)
- Test generation (coverage, edge cases)
- Bug fixing (accuracy, minimal changes)
- Documentation (clarity, completeness)
- Architecture (design quality)

**Files**:
- `crates/merlin-routing/src/benchmarks/capabilities.rs` (700 lines)
- `benchmarks/capability_suite/` (5000+ lines of test cases)

---

## Phase 9: Performance & Scale
**Timeline**: 4-6 weeks
**Priority**: Medium (Optimization)

### 9.1 Incremental Context Updates

**Problem**: Rebuilding entire context on each request

**Solution**: Track changes and update incrementally
```rust
pub struct IncrementalContextManager {
    current: Context,
    file_hashes: HashMap<PathBuf, u64>,
    watcher: FileWatcher,
}

impl IncrementalContextManager {
    pub async fn update(&mut self) -> Result<()> {
        // Get changed files since last update
        let changes = self.watcher.poll_changes();

        for change in changes {
            match change.kind {
                ChangeKind::Modified => {
                    // Re-index only changed file
                    let content = fs::read_to_string(&change.path)?;
                    self.current.update_file(change.path, content);
                }
                ChangeKind::Created => {
                    self.current.add_file(change.path);
                }
                ChangeKind::Deleted => {
                    self.current.remove_file(&change.path);
                }
            }
        }

        Ok(())
    }

    pub fn get_context(&self) -> &Context {
        &self.current
    }
}
```

**Performance Impact**:
- Full rebuild: 2-5 seconds
- Incremental update: 50-200ms
- **10-25x faster**

**Files**:
- `crates/merlin-context/src/incremental.rs` (500 lines)

### 9.2 Parallel Task Execution

**Problem**: Sequential task execution is slow

**Solution**: Execute independent tasks in parallel
```rust
pub struct ParallelExecutor {
    pool: ThreadPool,
    max_concurrent: usize,
}

impl ParallelExecutor {
    pub async fn execute_graph(&self, graph: TaskGraph) -> Result<Vec<TaskResult>> {
        let batches = graph.topological_batches();
        let mut results = Vec::new();

        for batch in batches {
            // Execute all tasks in batch concurrently
            let batch_futures: Vec<_> = batch.iter()
                .map(|task| self.execute_task(task))
                .collect();

            let batch_results = try_join_all(batch_futures).await?;
            results.extend(batch_results);
        }

        Ok(results)
    }

    async fn execute_task(&self, task: &Task) -> Result<TaskResult> {
        // Each task gets its own executor in the pool
        self.pool.spawn(async move {
            let executor = AgentExecutor::new(/* ... */);
            executor.execute_streaming(task.clone()).await
        }).await
    }
}
```

**Safety**:
- File locking prevents concurrent writes
- Read-only operations fully parallel
- Write batching for efficiency

**Performance**:
- 5 independent tasks: 10s → 2s (5x faster)
- 10 independent tasks: 20s → 2s (10x faster)

**Files**:
- `crates/merlin-routing/src/executor/parallel.rs` (400 lines)

### 9.3 Response Streaming Optimization

**Problem**: Long wait before seeing output

**Solution**: Stream tokens as they're generated
```rust
pub struct StreamingOptimizer {
    buffer_size: usize,
    flush_interval: Duration,
}

impl StreamingOptimizer {
    pub async fn stream_response<S>(&self, stream: S) -> impl Stream<Item = Chunk>
    where
        S: Stream<Item = Token>,
    {
        stream
            .chunks_timeout(self.buffer_size, self.flush_interval)
            .map(|chunk| self.process_chunk(chunk))
    }

    fn process_chunk(&self, tokens: Vec<Token>) -> Chunk {
        // Combine tokens into coherent chunks
        let text = tokens.into_iter().map(|t| t.text).collect::<String>();

        Chunk {
            text,
            metadata: ChunkMetadata {
                tokens: tokens.len(),
                latency: self.measure_latency(),
            },
        }
    }
}
```

**UX Impact**:
- Time to first token: 200-500ms (was 2-5s)
- Perceived latency: -70%

**Files**:
- `crates/merlin-agent/src/streaming_optimizer.rs` (300 lines)

### 9.4 Caching Improvements

**Problem**: Current cache only checks exact matches

**Solution**: Hierarchical caching with partial matches
```rust
pub struct HierarchicalCache {
    l1: HashMap<String, Response>,           // Exact matches
    l2: HashMap<String, Response>,           // High similarity (0.95+)
    l3: HashMap<String, Response>,           // Moderate similarity (0.80+)
    embeddings: EmbeddingModel,
}

impl HierarchicalCache {
    pub async fn get(&mut self, query: &str) -> Option<CachedResponse> {
        // L1: Exact match (0ms)
        if let Some(resp) = self.l1.get(query) {
            return Some(CachedResponse::Exact(resp.clone()));
        }

        // L2: High similarity (5ms)
        let embedding = self.embeddings.embed(query).await;
        if let Some((key, score)) = self.find_similar(&embedding, 0.95) {
            return Some(CachedResponse::Similar {
                response: self.l2[key].clone(),
                similarity: score,
            });
        }

        // L3: Moderate similarity (10ms)
        if let Some((key, score)) = self.find_similar(&embedding, 0.80) {
            // Partial match - use as starting point
            return Some(CachedResponse::Partial {
                base: self.l3[key].clone(),
                similarity: score,
                needs_refinement: true,
            });
        }

        None
    }
}
```

**Hit Rates**:
- L1 (exact): 30-40%
- L2 (high sim): 20-30%
- L3 (moderate): 10-20%
- **Total: 60-90% cache hit rate**

**Files**:
- `crates/merlin-routing/src/cache/hierarchical.rs` (500 lines)

---

## Phase 10: Adaptive Intelligence
**Timeline**: 6-8 weeks
**Priority**: Medium (Future-proofing)

### 10.1 Learning from Feedback

**Problem**: Agent doesn't learn from mistakes

**Solution**: Feedback loop with reinforcement learning
```rust
pub struct FeedbackLearner {
    examples: Vec<Example>,
    model: PolicyModel,
}

pub struct Example {
    pub query: String,
    pub context: Context,
    pub response: Response,
    pub feedback: Feedback,
}

pub enum Feedback {
    Accept,                    // User accepted as-is
    Reject { reason: String }, // User rejected
    Modify { diff: Diff },     // User edited response
}

impl FeedbackLearner {
    pub async fn learn_from_feedback(&mut self, example: Example) {
        // Store example
        self.examples.push(example.clone());

        // Update policy model
        match &example.feedback {
            Feedback::Accept => {
                self.model.reward(example.query, example.response, 1.0);
            }
            Feedback::Reject { reason } => {
                self.model.penalize(example.query, example.response, -1.0);
                // Learn from reason
                self.model.add_constraint(reason);
            }
            Feedback::Modify { diff } => {
                // Learn from the correction
                self.model.update_from_diff(
                    example.response,
                    diff,
                    0.5,  // Partial credit
                );
            }
        }
    }

    pub fn apply_learnings(&self, context: &mut Context) {
        // Adjust context based on past feedback
        let similar_examples = self.find_similar_examples(context);
        for example in similar_examples {
            context.add_constraint(example.learned_pattern());
        }
    }
}
```

**Integration**:
- After each response, ask: "Was this helpful? (y/n/edit)"
- Store feedback with context
- Periodically retrain policy

**Files**:
- `crates/merlin-agent/src/feedback_learner.rs` (600 lines)

### 10.2 Dynamic Prompt Engineering

**Problem**: Fixed prompts don't adapt to context

**Solution**: Generate prompts dynamically based on task
```rust
pub struct DynamicPromptGenerator {
    templates: HashMap<TaskCategory, PromptTemplate>,
    examples: ExampleBank,
}

impl DynamicPromptGenerator {
    pub fn generate(&self, task: &Task, context: &Context) -> String {
        let template = &self.templates[&task.category];

        // Select relevant examples (few-shot learning)
        let examples = self.examples.find_similar(task, 3);

        // Build prompt
        let mut prompt = template.base.clone();

        // Add task-specific instructions
        prompt.push_str(&self.generate_task_instructions(task));

        // Add examples
        for example in examples {
            prompt.push_str(&format!(
                "\n\nExample:\nInput: {}\nOutput: {}",
                example.input,
                example.output
            ));
        }

        // Add context hints
        prompt.push_str(&self.generate_context_hints(context));

        prompt
    }

    fn generate_task_instructions(&self, task: &Task) -> String {
        match task.category {
            TaskCategory::CodeGeneration => {
                format!(
                    "Generate {} code that {}. Ensure it follows the project's style guide.",
                    task.language,
                    task.requirements
                )
            }
            TaskCategory::Refactoring => {
                format!(
                    "Refactor {} to improve {}. Preserve behavior and maintain tests.",
                    task.target_code,
                    task.improvement_goals
                )
            }
            _ => task.description.clone(),
        }
    }
}
```

**Files**:
- `crates/merlin-agent/src/dynamic_prompts.rs` (700 lines)

### 10.3 Multi-Language Support

**Problem**: Only Rust is deeply supported

**Solution**: Pluggable language backends
```rust
pub trait LanguageBackend: Send + Sync {
    fn name(&self) -> &str;
    fn extensions(&self) -> &[&str];

    async fn parse(&self, code: &str) -> Result<AST>;
    async fn format(&self, code: &str) -> Result<String>;
    async fn lint(&self, code: &str) -> Result<Vec<Diagnostic>>;
    async fn find_symbol(&self, name: &str, files: &[PathBuf]) -> Result<Vec<Location>>;
    async fn get_dependencies(&self, file: &Path) -> Result<Vec<Dependency>>;
}

pub struct LanguageRegistry {
    backends: HashMap<String, Arc<dyn LanguageBackend>>,
}

impl LanguageRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            backends: HashMap::new(),
        };

        // Register built-in backends
        registry.register(Arc::new(RustBackend::new()));
        registry.register(Arc::new(PythonBackend::new()));
        registry.register(Arc::new(TypeScriptBackend::new()));
        registry.register(Arc::new(GoBackend::new()));

        registry
    }

    pub fn get_backend(&self, language: &str) -> Option<&Arc<dyn LanguageBackend>> {
        self.backends.get(language)
    }
}
```

**Priority Languages**:
1. **Rust** (current, complete)
2. **Python** (80% of AI codebases)
3. **TypeScript/JavaScript** (web development)
4. **Go** (cloud infrastructure)
5. **C/C++** (systems programming)

**Implementation Per Language**:
- Parser integration (tree-sitter)
- LSP client for navigation
- Language-specific linting
- Test framework integration
- Build tool integration

**Files**:
- `crates/merlin-languages/src/python/mod.rs` (1500 lines)
- `crates/merlin-languages/src/typescript/mod.rs` (1500 lines)
- `crates/merlin-languages/src/go/mod.rs` (1200 lines)

### 10.4 Project Templates & Scaffolding

**Problem**: Starting new features is tedious

**Solution**: Smart scaffolding based on patterns
```rust
pub struct Scaffolder {
    templates: TemplateLibrary,
    pattern_detector: PatternDetector,
}

impl Scaffolder {
    pub async fn scaffold(
        &self,
        intent: &Intent,
        codebase: &Codebase,
    ) -> Result<Scaffold> {
        // Detect patterns in existing codebase
        let patterns = self.pattern_detector.detect(codebase);

        // Select appropriate template
        let template = self.templates.find_best_match(intent, &patterns);

        // Generate files
        let mut files = Vec::new();
        for file_template in template.files {
            let content = self.render_template(
                &file_template,
                intent,
                &patterns,
            );
            files.push(ScaffoldedFile {
                path: file_template.path,
                content,
            });
        }

        Ok(Scaffold { files })
    }
}
```

**Templates**:
- "Add REST endpoint" → Controller, route, tests, docs
- "Create database migration" → Migration file, model update, tests
- "Add CLI command" → Command definition, args parser, tests
- "Implement trait" → Impl block, required methods, tests

**Files**:
- `crates/merlin-agent/src/scaffolder.rs` (600 lines)
- `templates/` (100+ template files)

---

## Success Metrics

### Phase 6 Targets (Agent Intelligence)
- [ ] TypeScript tool syntax: 90% success rate on complex chains
- [ ] Tool chaining: 5-10x speedup on multi-file operations
- [ ] Doc tracking: Catch 95% of stale docs
- [ ] Style enforcement: 99% consistency with codebase patterns

### Phase 7 Targets (Context Mastery)
- [ ] Context pruning: Reduce tokens by 60-80% while maintaining quality
- [ ] Adaptive windows: 90% of tasks fit in optimal window
- [ ] Semantic search: 95% relevance in top 5 results
- [ ] LSP navigation: 100% accuracy on "find definition/references"

### Phase 8 Targets (Multi-Model Orchestra)
- [ ] Cost reduction: 5x cheaper ($0.15 → $0.03 per request)
- [ ] Ensemble validation: 95% agreement on correct code
- [ ] Model selection: 90% accuracy in choosing right model

### Phase 9 Targets (Performance)
- [ ] Incremental updates: 10-25x faster context rebuilds
- [ ] Parallel execution: 5-10x speedup on independent tasks
- [ ] Streaming: 70% reduction in perceived latency
- [ ] Cache hit rate: 60-90% total hits (L1+L2+L3)

### Phase 10 Targets (Adaptive Intelligence)
- [ ] Feedback learning: 20% improvement after 100 examples
- [ ] Multi-language: 5 languages with 80%+ feature parity
- [ ] Scaffolding: 80% of generated code accepted without edits

### Overall Quality Targets
- [ ] Test coverage: 70% (from 35%)
- [ ] Success rate: 95% (from 85%)
- [ ] P95 latency: < 3s for simple tasks
- [ ] Daily cost: < $1.00 per user

---

## Risk Mitigation

### TypeScript Tool Syntax (Phase 6.1)
**Risk**: Security vulnerabilities in JS runtime
**Mitigation**:
- Sandboxed execution (deno_core with permissions disabled)
- Static analysis before execution
- Timeout and resource limits
- Extensive fuzzing

**Risk**: Models generate invalid TypeScript
**Mitigation**:
- Type checking before execution
- Clear error messages with examples
- Fallback to JSON syntax on parse failure

### Context Pruning (Phase 7.1)
**Risk**: Pruning removes critical information
**Mitigation**:
- Conservative thresholds initially
- Always include explicitly mentioned files
- User override: "include file X"
- Logging to debug pruning decisions

### Model Specialization (Phase 8.1)
**Risk**: Wrong model selection degrades quality
**Mitigation**:
- Benchmark all models on capability suite
- Conservative fallback: use best model when uncertain
- User override: "use Claude for this"
- A/B testing for model selection

### Parallel Execution (Phase 9.2)
**Risk**: Race conditions and data corruption
**Mitigation**:
- File locking for all writes
- Transaction logs for rollback
- Conflict detection before merging
- Extensive concurrency testing

### Learning from Feedback (Phase 10.1)
**Risk**: Bad feedback corrupts learned policies
**Mitigation**:
- Weight by feedback confidence
- Detect contradictory feedback
- Allow policy reset
- Human review of major policy changes

---

## Implementation Timeline

### Months 1-2: Phase 6 (Agent Intelligence)
- Week 1-2: TypeScript tool runtime + parser
- Week 3-4: Smart tool chaining
- Week 5-6: Documentation tracking
- Week 7-8: Coding standards enforcement

### Months 3-4: Phase 7 (Context Mastery)
- Week 9-10: Context pruning + knapsack optimizer
- Week 11-12: Adaptive context windows
- Week 13-14: Conversation summarization
- Week 15-16: Semantic code search + LSP integration

### Months 5-6: Phase 8 (Multi-Model Orchestra)
- Week 17-18: Task-specific routing
- Week 19-20: Ensemble validation
- Week 21-22: Model capability benchmarks
- Week 23-24: Cost optimization

### Months 7-8: Phase 9 (Performance & Scale)
- Week 25-26: Incremental context updates
- Week 27-28: Parallel task execution
- Week 29-30: Streaming optimization
- Week 31-32: Hierarchical caching

### Months 9-10: Phase 10 (Adaptive Intelligence)
- Week 33-34: Feedback learning system
- Week 35-36: Dynamic prompt engineering
- Week 37-38: Multi-language support (Python, TypeScript)
- Week 39-40: Project scaffolding

**Total Timeline**: 10 months
**Estimated Effort**: 800-1000 hours

---

## Conclusion

This roadmap transforms Merlin from a capable multi-model router into a **world-class AI coding agent** that:

1. **Understands code deeply** (LSP integration, semantic search)
2. **Uses tools naturally** (TypeScript syntax, smart chaining)
3. **Manages context intelligently** (pruning, adaptive windows, summarization)
4. **Optimizes cost** (task-specific models, caching, parallel execution)
5. **Learns continuously** (feedback loops, dynamic prompts)
6. **Scales to any language** (pluggable backends)

The **TypeScript tool syntax** (Phase 6.1) is the most revolutionary change - it leverages LLMs' training on billions of lines of open-source code to make tool usage completely natural. Combined with intelligent context management and multi-model orchestration, Merlin will set a new standard for AI-assisted development.

**Key Differentiators**:
- ✅ Local-first (Qwen 7B for privacy + speed)
- ✅ Cost-optimized (5x cheaper with model specialization)
- ✅ Parallel execution (10x faster on multi-file operations)
- ✅ Language-aware (LSP integration for precise navigation)
- ✅ Learning system (improves from feedback)

**Next Actions**:
1. Review and refine this plan with stakeholders
2. Begin Phase 6.1: TypeScript tool syntax prototype
3. Set up benchmarking infrastructure for measuring improvements
4. Establish baseline metrics for comparison

This is an ambitious but achievable plan to build the best AI coding agent in the world. 🚀
