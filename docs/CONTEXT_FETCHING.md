# Context Fetching Strategy

## Overview

This document outlines the strategy for intelligent context gathering - automatically discovering and including relevant files in the context sent to the LLM, even when they're not explicitly mentioned in the user's query.

**Goal**: When a user says "Implement authentication", the system should automatically:
1. Identify the main entry point(s)
2. Find all authentication-related code
3. Discover related types, traits, and implementations
4. Include dependencies and interfaces
5. Add relevant test files
6. Provide enough context for the LLM to make informed changes

## Current State vs. Target State

### ✅ Current Implementation (COMPLETED)
- ✅ **Intelligent query analysis**: Heuristic-based QueryAnalyzer extracts intent from natural language
- ✅ **Local subagent**: Ollama-based ContextAgent generates structured ContextPlans
- ✅ **Semantic search**: Symbol and reference finding using rust-analyzer backend
- ✅ **Graph traversal**: Import traversal with configurable depth limits
- ✅ **Relevance ranking**: Keyword-based scoring and sorting of files
- ✅ **Context expansion**: ContextExpander executes plans with multiple strategies
- ✅ **Automatic fallback**: Gracefully falls back to basic scanning if subagent unavailable

### Implementation Status
**Phase 1-3 Complete**: The system now uses a hybrid approach with local subagent (Ollama) for intelligent context planning, combined with heuristic fallbacks.

## ✅ Implemented Architecture

### Subagent Approach with Local LLM (CHOSEN & IMPLEMENTED)

We use a lightweight "Context Agent" powered by Ollama that runs before the main LLM:

```
User Query → Context Agent → Context Builder → Main LLM
              (small model)    (gathers files)   (generates code)
```

**Context Agent Responsibilities:**
1. Extract key terms and symbols from the query
2. Determine search strategy (broad vs. focused)
3. Identify relevant file patterns
4. Specify semantic search queries
5. Return a `ContextPlan` with specific instructions

**Benefits:**
- Separates concerns (context discovery vs. code generation)
- Can use a smaller, faster model (Groq free tier, local model)
- Cheaper overall (small model + targeted context)
- Allows for iterative refinement

**Implementation:**
```rust
pub struct ContextPlan {
    /// Keywords extracted from query
    pub keywords: Vec<String>,
    /// Specific symbols to search for
    pub symbols_to_find: Vec<String>,
    /// File patterns to include (e.g., "auth", "user", "session")
    pub file_patterns: Vec<String>,
    /// Whether to include tests
    pub include_tests: bool,
    /// Maximum context depth (how far to traverse)
    pub max_depth: usize,
    /// Expansion strategy
    pub strategy: ExpansionStrategy,
}

pub enum ExpansionStrategy {
    /// Find specific symbols and their direct dependencies
    Focused { symbols: Vec<String> },
    /// Broad search across files matching patterns
    Broad { patterns: Vec<String> },
    /// Start from entry points and expand
    EntryPointBased { entry_files: Vec<PathBuf> },
    /// Search semantically for related concepts
    Semantic { query: String, top_k: usize },
}
```

### ✅ Heuristic Fallback (IMPLEMENTED)

The system includes a heuristic-based `QueryAnalyzer` that works without the subagent:

**Implemented Features:**
- ✅ Keyword extraction with stop-word filtering
- ✅ Entity detection (capitalized words, Rust paths)
- ✅ Action classification (Create, Modify, Debug, Explain, Refactor, Search)
- ✅ Scope detection (Focused, Module, Codebase)
- ✅ Complexity estimation (Simple, Medium, Complex)

**Benefits:**
- Works immediately without Ollama
- Fast and predictable
- No API costs
- Provides baseline context when subagent unavailable

## ✅ Implementation Strategy: Hybrid Approach (COMPLETED)

We implemented a hybrid approach that combines both methods:
1. ✅ **Phase 1**: Heuristic QueryAnalyzer extracts initial intent (DONE)
2. ✅ **Phase 2**: Local subagent (Ollama) generates detailed ContextPlan (DONE)
3. ✅ **Phase 3**: ContextExpander executes plan with semantic analysis (DONE)
4. ⏳ **Phase 4**: Learn from user feedback to improve heuristics (FUTURE)

## Context Expansion Pipeline

```
┌─────────────────┐
│  User Query     │
│  "Implement     │
│  authentication"│
└────────┬────────┘
         │
         ▼
┌─────────────────────────────┐
│  1. Query Analysis          │
│  - Extract: "authentication"│
│  - Intent: Implementation   │
│  - Complexity: Medium-High  │
└────────┬────────────────────┘
         │
         ▼
┌──────────────────────────────┐
│  2. Initial Seed Discovery   │
│  - Scan for files: *auth*   │
│  - Search symbols: Auth*    │
│  - Entry point: main.rs     │
└────────┬─────────────────────┘
         │
         ▼
┌────────────────────────────────┐
│  3. Semantic Expansion         │
│  - Use rust-analyzer:          │
│    • find_definition()         │
│    • find_references()         │
│    • get_related_context()     │
│  - Follow import chains        │
│  - Identify trait impls        │
└────────┬───────────────────────┘
         │
         ▼
┌──────────────────────────────────┐
│  4. Dependency Resolution        │
│  - Add imported modules          │
│  - Include trait definitions     │
│  - Add related types/structs     │
└────────┬─────────────────────────┘
         │
         ▼
┌──────────────────────────────────┐
│  5. Relevance Ranking            │
│  - Score by:                     │
│    • Symbol match count          │
│    • Import distance             │
│    • File age/frequency          │
│    • Semantic similarity         │
└────────┬─────────────────────────┘
         │
         ▼
┌──────────────────────────────────┐
│  6. Context Assembly             │
│  - Sort by relevance             │
│  - Truncate to token limit       │
│  - Add system prompt             │
│  - Include file summaries        │
└────────┬─────────────────────────┘
         │
         ▼
┌──────────────────────────────────┐
│  7. Pass to Main LLM             │
└──────────────────────────────────┘
```

## Key Components to Implement

### 1. Query Analyzer
```rust
pub struct QueryAnalyzer {
    /// Extract intent from natural language
    fn analyze(&self, query: &str) -> QueryIntent;
}

pub struct QueryIntent {
    pub action: Action,  // Create, Modify, Debug, Explain
    pub keywords: Vec<String>,
    pub entities: Vec<String>,  // Types, functions mentioned
    pub scope: Scope,  // Single file, Module, Codebase-wide
}
```

### 2. Context Expander
```rust
pub struct ContextExpander {
    backend: Box<dyn LanguageProvider>,
    
    /// Start with seed files and expand to related files
    fn expand(&self, seeds: Vec<PathBuf>, plan: &ContextPlan) -> Vec<FileContext>;
    
    /// Follow imports recursively up to max_depth
    fn traverse_imports(&self, file: &Path, depth: usize) -> Vec<PathBuf>;
    
    /// Find all files that reference a symbol
    fn find_symbol_references(&self, symbol: &str) -> Vec<PathBuf>;
}
```

### 3. Relevance Scorer
```rust
pub struct RelevanceScorer {
    /// Score a file's relevance to the query
    fn score(&self, file: &FileContext, query: &QueryIntent) -> f64;
    
    /// Factors: keyword matches, recency, import distance, symbol density
    fn calculate_score(&self, file: &FileContext, factors: &ScoringFactors) -> f64;
}
```

### 4. Token Budget Manager
```rust
pub struct TokenBudgetManager {
    max_tokens: usize,
    
    /// Fit as many relevant files as possible within budget
    fn fit_to_budget(&self, files: Vec<FileContext>) -> Vec<FileContext>;
    
    /// Estimate tokens for a file
    fn estimate_tokens(&self, file: &FileContext) -> usize;
    
    /// Summarize less relevant files instead of including full content
    fn summarize_if_needed(&self, file: &FileContext) -> FileContext;
}
```

## Concrete Examples

### Example 1: "Implement authentication"

**Query Analysis:**
- Action: Create/Implement
- Keywords: ["authentication", "auth", "login", "session"]
- Scope: Codebase-wide

**Context Expansion:**
1. **Seed Discovery**: Search for files matching "*auth*", "*login*", "*session*"
   - Found: `src/auth/mod.rs`, `src/models/user.rs`, `src/middleware/auth.rs`

2. **Semantic Expansion**: 
   - Find all structs/traits related to "User", "Session", "Token"
   - Traverse imports to find dependencies
   - Include: `src/config.rs` (for auth config), `src/database/user_repo.rs`

3. **Related Patterns**:
   - Find middleware pattern files
   - Include router/endpoint definitions
   - Add: `src/routes/mod.rs`, `src/main.rs` (to see app structure)

4. **Test Context**:
   - Include: `tests/auth_tests.rs`, `tests/integration/login_test.rs`

**Final Context** (ranked by relevance):
1. `src/auth/mod.rs` (100% - direct match)
2. `src/models/user.rs` (95% - core entity)
3. `src/middleware/auth.rs` (90% - implementation location)
4. `src/database/user_repo.rs` (85% - data layer)
5. `src/routes/mod.rs` (80% - integration point)
6. `src/config.rs` (70% - configuration)
7. `src/main.rs` (summary only - 60% - context)
8. `tests/auth_tests.rs` (60% - examples)

### Example 2: "Fix the bug in UserService.find_by_email"

**Query Analysis:**
- Action: Debug/Fix
- Keywords: ["bug", "UserService", "find_by_email"]
- Scope: Focused (single method)

**Context Expansion:**
1. **Direct Symbol Search**:
   - Find definition of `UserService::find_by_email`
   - Found: `src/services/user_service.rs:45`

2. **Minimal Expansion**:
   - Include the full `UserService` struct and impl block
   - Add `User` type definition (referenced)
   - Include any helper functions called within `find_by_email`

3. **Related Context** (small):
   - Tests that call `find_by_email`
   - Database schema if using raw SQL

**Final Context** (focused, minimal):
1. `src/services/user_service.rs` (lines 1-100)
2. `src/models/user.rs` (type definition only)
3. `tests/user_service_test.rs` (relevant test)

## ✅ Implementation Checklist

### ✅ Phase 1: Foundation (COMPLETED)
- ✅ **Create query analysis module**
  - ✅ Implement `QueryAnalyzer` struct (`src/query/analyzer.rs`)
  - ✅ Add keyword extraction with stop-word filtering
  - ✅ Add action classification (Create/Modify/Debug/Explain/Refactor/Search)
  - ✅ Add scope detection (Focused/Module/Codebase)
  - ✅ Add complexity estimation (Simple/Medium/Complex)
  - ✅ Write unit tests for query analysis

- ✅ **Enhance ContextBuilder with expansion capabilities**
  - ✅ Add `ContextPlan` type (`src/query/types.rs`)
  - ✅ Implement seed file discovery via pattern matching
  - ✅ Add file pattern matching in `ContextExpander`
  - ✅ Add symbol search integration
  - ✅ Integrate with ContextBuilder

- ✅ **Implement graph traversal**
  - ✅ Add `expand_from_entry_points()` method with depth limiting
  - ✅ Import traversal using `extract_imports()` from backend
  - ✅ Implement depth-limited BFS with HashSet for cycle detection
  - ✅ Integrated into ContextExpander

### ✅ Phase 2: Semantic Analysis Integration (COMPLETED)
- ✅ **Enable rust-analyzer semantic features**
  - ✅ Removed old `enhance_with_semantic_context()` method
  - ✅ Implement symbol search using backend's `search_symbols()`
  - ✅ Implement reference finding via `SearchQuery` with `include_references`
  - ✅ Add related context using backend's `get_related_context()`
  - ✅ Integrated into `ContextExpander::expand_focused()`

- ✅ **Implement relevance scoring**
  - ✅ Keyword-based scoring in `ContextExpander::expand()`
  - ✅ Path and content matching with weighted scores
  - ✅ Sort files by relevance (most relevant first)
  - ✅ Integrated into expansion pipeline

- ⏳ **Add token budget management** (PARTIAL)
  - ✅ Basic truncation using `max_files` limit
  - ⏳ Token estimation (using rough char count)
  - ⏳ File summarization for low-priority files (FUTURE)
  - ⏳ Adaptive truncation based on token budget (FUTURE)

### ✅ Phase 3: Context Agent (COMPLETED)
- ✅ **Create context agent module**
  - ✅ Design `ContextAgent` trait (`src/subagent/agent.rs`)
  - ✅ Implement local model integration with Ollama (`src/subagent/local.rs`)
  - ✅ Using `ollama-rs` crate for clean API integration
  - ✅ Create prompt templates for context planning (`src/subagent/prompts.rs`)
  - ✅ Implement `ContextPlan` generation with JSON parsing
  - ✅ Add fallback to heuristics if agent fails (automatic in ContextBuilder)
  - ⏳ Write tests for agent (FUTURE)

- ✅ **Integrate context agent with main flow**
  - ✅ Add agent call in `ContextBuilder::build_context()`
  - ✅ Query analysis before agent invocation
  - ✅ Parse agent response into `ContextPlan` (JSON with markdown extraction)
  - ✅ Use plan to guide context expansion via `ContextExpander`
  - ✅ Graceful fallback to basic file scanning
  - ✅ Backend initialization on-demand
  - ✅ Comprehensive logging for debugging
  - ⏳ Implement caching for repeated queries (FUTURE)

### ⏳ Phase 4: Optimization & Polish (FUTURE)
- ⏳ **Performance optimization**
  - ⏳ Profile context building performance
  - ⏳ Add caching for file analysis
  - ⏳ Optimize semantic queries (batch processing)
  - [ ] Parallelize independent operations
  - [ ] Add progressive loading (start with high-confidence files)

- [ ] **User experience improvements**
  - [ ] Add `--explain-context` flag to show what was included and why
  - [ ] Add `--max-context-files` override
  - [ ] Implement context preview before sending to LLM
  - [ ] Add user feedback loop (was context sufficient?)
  - [ ] Create context inclusion hints in output

- [ ] **Testing & validation**
  - [ ] Create test scenarios for different query types
  - [ ] Add benchmark suite for context quality
  - [ ] Measure precision/recall of file inclusion
  - [ ] Test with real-world queries
  - [ ] Add integration tests for full pipeline

### Phase 5: Advanced Features (Future)
- [ ] **Embeddings-based semantic search**
  - [ ] Generate embeddings for all files
  - [ ] Add vector similarity search
  - [ ] Use for "related files" discovery
  - [ ] Periodically update embeddings

- [ ] **Learning from feedback**
  - [ ] Track which files were actually modified
  - [ ] Learn common query→file patterns
  - [ ] Improve heuristics based on usage
  - [ ] Add personalization per project

- [ ] **Multi-language support**
  - [ ] Add Java backend context expansion
  - [ ] Add Python backend context expansion
  - [ ] Add TypeScript backend context expansion
  - [ ] Implement language-specific strategies

## ✅ Implementation Summary

### What Was Built

The intelligent context fetching system is now **fully operational** with the following components:

**Core Modules:**
- `crates/agentic-context/src/query/` - Query analysis and intent extraction
- `crates/agentic-context/src/subagent/` - Ollama-based context planning agent
- `crates/agentic-context/src/expander.rs` - Context expansion execution engine
- `crates/agentic-context/src/builder.rs` - Integrated context building pipeline

**Key Features:**
- ✅ Heuristic query analysis (works without Ollama)
- ✅ Local LLM-based context planning (Ollama integration)
- ✅ Multiple expansion strategies (Focused, Broad, EntryPoint, Semantic)
- ✅ Semantic symbol search via rust-analyzer
- ✅ Import graph traversal with depth limiting
- ✅ Relevance-based file ranking
- ✅ Automatic test file discovery
- ✅ Graceful fallback to basic scanning

### Setup Instructions

**1. Install Ollama:**
```bash
# Install Ollama from https://ollama.ai
# Or use package manager
curl -fsSL https://ollama.ai/install.sh | sh
```

**2. Pull the model:**
```bash
ollama pull qwen2.5-coder:7b
```

**3. Start Ollama server:**
```bash
ollama serve
```

**4. Run agentic with intelligent context:**
```bash
# The subagent will automatically activate when a language backend is enabled
cargo run --bin agentic -- prompt "Implement authentication" --max-files 10
```

**5. Configure (optional):**
```bash
# Set custom Ollama host
export OLLAMA_HOST="http://localhost:11434"

# Set custom model
export OLLAMA_MODEL="qwen2.5-coder:7b"
```

### How It Works

```
1. User: "Implement authentication"
   ↓
2. QueryAnalyzer extracts: action=Create, keywords=[auth, user], complexity=Complex
   ↓
3. LocalContextAgent (Ollama) generates ContextPlan:
   - keywords: ["auth", "user", "session"]
   - symbols: ["Auth", "User", "Session"]
   - patterns: ["auth", "login", "user"]
   - strategy: Focused
   ↓
4. ContextExpander executes plan:
   - Finds files matching patterns
   - Searches for symbols via rust-analyzer
   - Traverses imports
   - Ranks by relevance
   ↓
5. Returns optimized context to main LLM
```

## Success Metrics

**Target Goals:**
1. **Context Relevance**: >80% of included files are actually used/modified
2. **Coverage**: <5% of queries result in "I need more context"
3. **Efficiency**: Average context size < 50% of max tokens
4. **Speed**: Context building < 3 seconds for typical queries
5. **Cost**: Context agent cost = $0 (local Ollama)

**Current Status:** ✅ All core functionality implemented and ready for testing

## Future Enhancements

1. ⏳ **Embeddings-based semantic search** - Vector similarity for "related files"
2. ⏳ **Learning from feedback** - Track which files were actually modified
3. ⏳ **Multi-language support** - Java, Python, TypeScript backends
4. ⏳ **Query caching** - Cache context plans for repeated queries
5. ⏳ **Token budget management** - Adaptive truncation and file summarization
6. ⏳ **Context explanation** - `--explain-context` flag to show reasoning

## References

- **Implementation**: `crates/agentic-context/src/`
- **Query Analysis**: `crates/agentic-context/src/query/`
- **Subagent**: `crates/agentic-context/src/subagent/`
- **Expander**: `crates/agentic-context/src/expander.rs`
- **Language Backends**: `crates/agentic-languages/`
- **Rust Backend**: `crates/agentic-languages/languages/rust-backend/`
- **Related Docs**: `ARCHITECTURE.md`, `PHASES.md`
