# Agentic Optimizer - Cost Reduction Plan

## Executive Summary

**Current Costs (3-day analysis):**
- **Total**: $45.61 ($456/month projected)
- **Cache Reads**: $27.76 (60.8%) - 92.5M tokens
- **Cache Writes**: $14.36 (31.5%) - 3.8M tokens  
- **Output Tokens**: $3.44 (7.5%) - 229K tokens
- **Input Tokens**: $0.05 (0.1%) - 17K tokens

**Primary Cost Drivers:**
1. Cache reads dominate (60.8% of spend)
2. Cache writes substantial (31.5% of spend)
3. Output tokens significant (7.5% of spend)

**Target Savings: 70-85% reduction ($318-$388/month)**

---

## Cost Breakdown Analysis

### Per-Day Averages
- Cache reads: ~30.8M tokens/day → $9.25/day
- Cache writes: ~1.27M tokens/day → $4.79/day
- Output tokens: ~76.5K tokens/day → $1.15/day
- **Average daily cost: $15.20**

### Problem Identification
1. **Excessive cache reads**: Repeated re-processing of same codebase context
2. **Large cache writes**: Full codebase context likely being cached each request
3. **Verbose outputs**: 178K output tokens on Oct 1 suggests excessive generation
4. **Poor context reuse**: High cache write-to-read ratio indicates inefficient caching

---

## Optimization Strategies

### Strategy 1: Intelligent Context Windowing (Est. 50-60% cost reduction)

**Problem**: Sending entire codebase context on every request causes massive cache overhead.

**Solution**: Implement smart context selection
- Use AST parsing to identify relevant files/functions only
- Implement dependency graph to include only necessary context
- Use vector embeddings for semantic similarity search
- Maintain minimal "core context" + focused "task context"

**Implementation**:
```
1. Index codebase on first run (one-time cost)
2. For each request:
   a. Parse user query to identify relevant symbols
   b. Use grep/ripgrep to find definitions
   c. Build minimal dependency tree (max 5-10 files)
   d. Send only relevant context (5-20KB vs full codebase)
```

**Cost Impact**:
- **Before**: 30.8M cache reads/day @ $0.3/M = $9.25/day
- **After**: 3-5M cache reads/day @ $0.3/M = $0.90-$1.50/day
- **Savings**: $7.75-$8.35/day ($232-$250/month)

- **Before**: 1.27M cache writes/day @ $3.75/M = $4.79/day
- **After**: 0.15-0.25M cache writes/day @ $3.75/M = $0.56-$0.94/day
- **Savings**: $3.85-$4.23/day ($115-$127/month)

**Total Strategy 1 Savings: $347-$377/month (76-83% reduction)**

---

### Strategy 2: Output Token Optimization (Est. 30-40% output reduction)

**Problem**: AI generates verbose responses, unnecessary explanations, repeated code.

**Solution**: Constrained generation with explicit instructions

**Implementation**:
```rust
// Add to system prompt
"CRITICAL RULES:
- Output ONLY code changes, no explanations unless explicitly asked
- Use edit_file tool instead of generating full files
- For multi-file changes, output file paths only, then edit on request
- Maximum response: 150 tokens for acknowledgments, 500 for code blocks
- Use references (e.g., 'see line 45') instead of repeating code"
```

**Additional Techniques**:
1. **Streaming with early termination**: Stop generation when task complete
2. **Diff-based edits**: Send only changed lines, not full files
3. **Response templates**: Predefined formats for common tasks
4. **Compressed notation**: Use shorthand in tool calls

**Cost Impact**:
- **Before**: 76.5K output tokens/day @ $15/M = $1.15/day
- **After**: 30-35K output tokens/day @ $15/M = $0.45-$0.52/day
- **Savings**: $0.63-$0.70/day ($19-$21/month)**

---

### Strategy 3: Multi-Turn Conversation Optimization (Est. 40% reduction)

**Problem**: Each turn re-caches similar context, accumulating costs.

**Solution**: Session-aware context management

**Implementation**:
```
1. Maintain conversation state in local cache
2. Track which files/context already sent
3. Use differential updates:
   - First turn: Send minimal context
   - Follow-ups: "Context unchanged, continue from previous"
4. Implement context expiry (5-10 min TTL)
5. Use prompt caching strategically:
   - Cache static system prompts (rules, tool definitions)
   - Cache unchanged file contents
   - DON'T cache dynamic conversation history
```

**Cost Impact**:
- Reduces redundant cache writes in multi-turn conversations
- **Savings**: Additional 20-30% on cache operations
- **Estimated**: $2-3/day ($60-$90/month)

---

### Strategy 4: Haiku/Sonnet Hybrid Approach (Est. 60-70% cost reduction)

**Problem**: Using Sonnet 4.5 for all tasks, including simple ones.

**Solution**: Route to cheaper models based on task complexity

**Model Costs**:
- **Haiku 4**: $0.80/M input, $4/M output, $0.08/M cache read
- **Sonnet 4.5**: $3/M input, $15/M output, $0.30/M cache read

**Routing Logic**:
```rust
enum TaskComplexity {
    Simple,   // Haiku: grep searches, file viewing, simple edits
    Medium,   // Haiku: single-file refactors, bug fixes
    Complex,  // Sonnet: architecture, multi-file changes, reasoning
}

fn route_model(task: &str) -> Model {
    if task.contains(&["explain", "design", "architecture"]) {
        Model::Sonnet45
    } else if task.contains(&["search", "find", "view", "show"]) {
        Model::Haiku4
    } else {
        // Default to Haiku, escalate if needed
        Model::Haiku4
    }
}
```

**Cost Impact** (assuming 70% tasks can use Haiku):
- **Haiku portion (70%)**:
  - Cache reads: 21.6M @ $0.08/M = $1.73/day (was $6.48)
  - Output: 53.5K @ $4/M = $0.21/day (was $0.80)
  - Savings: $5.34/day

- **Sonnet portion (30%)**: $4.56/day
- **Total**: $6.29/day vs $15.20/day
- **Savings**: $8.91/day ($267/month)**

---

### Strategy 5: Local + Multi-Cloud Hybrid (Est. 95-98% cost reduction)

**Problem**: Even cheap cloud APIs accumulate costs at scale; latency for simple tasks.

**Solution**: Intelligent tiered routing: Local → Groq (free) → Gemini Flash → Sonnet

**Architecture**:
```
Layer 1 (Local): Ultra-fast router (Phi-3-mini, 1-3B)
  ├─ Task classification (~10ms, $0)
  ├─ Simple searches/lookups ($0)
  └─ Route to Layer 2 or 3
  
Layer 2 (Local): Code specialist (Qwen2.5-Coder-7B)
  ├─ Code completions (~100ms, $0)
  ├─ Single-file edits ($0)
  ├─ Simple refactors ($0)
  └─ Escalate if uncertain

Layer 3 (Cloud Free): Groq API (Llama 3.1 70B)
  ├─ Multi-file analysis (~2s, ~$0 if under limit)
  ├─ Complex reasoning ($0.50/M if over limit)
  └─ Escalate if fails

Layer 4 (Cloud Cheap): Gemini Flash 2.0
  ├─ Advanced tasks (~3s, $0.30/M output)
  ├─ Cache: $0.01875/M (26x cheaper than Haiku)
  └─ Escalate for critical quality

Layer 5 (Cloud Premium): Sonnet 4.5
  └─ Only critical/complex tasks requiring best quality
```

**Hardware Requirements** (Consumer PC):
- **Minimum**: 8GB VRAM (RTX 3060) or 16GB RAM (CPU only)
  - Phi-3-mini (3.8B): Classification/routing
  - Qwen2.5-Coder-7B (quantized): Code tasks
  
- **Recommended**: 16GB VRAM (RTX 4060 Ti) or 32GB RAM
  - + DeepSeek-Coder-6.7B: Better code quality
  
- **Optimal**: 24GB VRAM (RTX 4090) or 64GB RAM
  - + Qwen2.5-Coder-32B (quantized): Near-Sonnet quality

**Local Model Performance**:

| Model | Size | VRAM | Speed (GPU) | Code Quality | Use Case |
|-------|------|------|-------------|--------------|----------|
| Phi-3-mini | 3.8B | 4GB | 100+ tok/s | 60% | Routing, classification |
| Qwen2.5-Coder-7B | 7B | 6GB | 50+ tok/s | 75% | Code edits, completion |
| DeepSeek-Coder-6.7B | 6.7B | 6GB | 50+ tok/s | 78% | Code understanding |
| Qwen2.5-Coder-32B | 32B | 20GB | 15+ tok/s | 88% | Complex refactors |

**Cloud API Alternatives** (cheaper than Haiku):

| Provider | Model | Input | Output | Cache Read | vs Haiku |
|----------|-------|-------|--------|------------|----------|
| **Groq** | Llama 3.1 70B | Free tier | Free tier | N/A | 100% cheaper |
| **Google** | Gemini Flash 2.0 | $0.075/M | $0.30/M | $0.01875/M | 90% cheaper |
| **DeepSeek** | DeepSeek-V3 | $0.27/M | $1.10/M | N/A | 72% cheaper |
| **OpenAI** | GPT-4o-mini | $0.15/M | $0.60/M | N/A | 81% cheaper |
| Anthropic | Haiku 4 | $0.80/M | $4.00/M | $0.08/M | baseline |

**Implementation**:
```rust
// Local model manager
struct LocalModels {
    router: Phi3Mini,      // 3.8B - instant classification
    coder: Qwen2_5Coder7B, // 7B - code specialist
}

// Multi-tier routing
async fn route_query(query: &str, models: &LocalModels) -> Response {
    // Layer 1: Local routing (10ms, $0)
    let task_type = models.router.classify(query).await;
    
    match task_type {
        TaskType::SimpleSearch | TaskType::Lookup => {
            // Handle locally, no API call
            handle_local(query)
        }
        TaskType::CodeEdit | TaskType::Completion => {
            // Layer 2: Local 7B model (100ms, $0)
            let result = models.coder.generate(query).await;
            if result.confidence > 0.85 {
                return result;
            }
            // Escalate if uncertain
            route_to_cloud(query, CloudTier::Groq)
        }
        TaskType::Complex => {
            // Start with free tier
            route_to_cloud(query, CloudTier::Groq)
        }
    }
}

async fn route_to_cloud(query: &str, tier: CloudTier) -> Response {
    let result = match tier {
        CloudTier::Groq => {
            // Try free Groq first
            groq_client.call(query).await
        }
        CloudTier::GeminiFlash => {
            // 4x cheaper than Haiku
            gemini_client.call(query).await
        }
        CloudTier::Sonnet => {
            // Last resort, highest quality
            sonnet_client.call(query).await
        }
    };
    
    // Auto-escalate on failure
    if result.is_err() || result.confidence < 0.7 {
        let next_tier = tier.escalate();
        route_to_cloud(query, next_tier).await
    } else {
        result
    }
}
```

**Cost Impact** (Full Hybrid):

Assume task distribution:
- 30% handled by local router (free)
- 40% handled by local 7B model (free)
- 15% routed to Groq (free tier)
- 10% routed to Gemini Flash ($0.30/M output)
- 5% routed to Sonnet ($15/M output)

**Before** (current):
- 76.5K output tokens/day @ $15/M = $1.15/day
- 30.8M cache reads/day @ $0.30/M = $9.25/day
- **Total: $15.20/day**

**After** (full hybrid):
- Local (70%): $0.00/day
- Groq (15%): $0.00/day (under free limit)
- Gemini (10%): 7.6K tokens @ $0.30/M = $0.002/day
- Gemini cache: 3.1M @ $0.01875/M = $0.06/day
- Sonnet (5%): 3.8K tokens @ $15/M = $0.06/day
- Sonnet cache: 1.5M @ $0.30/M = $0.45/day
- **Total: $0.57/day**

**Savings: $14.63/day ($439/month, 96.2% reduction)**

**Additional Benefits**:
- **Speed**: Local models respond in 10-200ms (vs 2-5s cloud)
- **Privacy**: Sensitive code never leaves your machine
- **Reliability**: No API rate limits or outages
- **Offline**: Works without internet for 70% of tasks

**Setup Requirements**:
```bash
# Install Ollama for local model management
# Windows: Download from ollama.ai
ollama pull phi3:mini        # 3.8B router
ollama pull qwen2.5-coder:7b # 7B code specialist

# Optional: Larger models if you have VRAM
ollama pull deepseek-coder:6.7b
ollama pull qwen2.5-coder:32b  # Requires 20GB+ VRAM
```

**Benchmark** (estimated on RTX 4060 Ti):
- Phi-3-mini (routing): ~150 tokens/sec, ~10ms latency
- Qwen2.5-Coder-7B: ~60 tokens/sec, ~100ms latency
- Total cost: $0 (electricity ~$0.15/month at 200W, $0.12/kWh)

---

### Strategy 6: Precomputed Indexing & Local Caching (Est. 20-30% reduction)

**Problem**: Repeatedly asking AI to find/search/analyze code structure.

**Solution**: Build local index, query locally, send only results to AI

**Implementation**:
```
1. On initialization:
   - Build symbol table (functions, structs, modules)
   - Create file dependency graph
   - Generate embeddings for semantic search
   - Extract doc comments and signatures

2. On user query:
   - Parse query locally (regex, AST)
   - Search index first (grep, symbol lookup)
   - Send only findings to AI for interpretation
   
3. Incremental updates:
   - Watch file changes
   - Update only modified files in index
```

**Cost Impact**:
- Eliminates 20-30% of AI requests entirely (pure lookups)
- **Savings**: $3-4.50/day ($90-$135/month)

---

## Recommended Architecture

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│                        User Query                            │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│              Query Analyzer & Router                         │
│  - Classify task complexity                                  │
│  - Determine required context                                │
│  - Select model (Haiku/Sonnet)                               │
└──────────────────────┬──────────────────────────────────────┘
                       │
         ┌─────────────┴─────────────┐
         │                           │
┌────────▼────────┐        ┌─────────▼────────┐
│  Local Index    │        │  Context Builder │
│  - Symbol table │        │  - AST parser    │
│  - Dep graph    │        │  - Dep resolver  │
│  - Embeddings   │        │  - Diff tracker  │
└────────┬────────┘        └─────────┬────────┘
         │                           │
         └─────────────┬─────────────┘
                       │
         ┌─────────────▼──────────────┐
         │    Minimal Context (5-20KB)│
         └─────────────┬──────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│                   Claude API Call                            │
│  - Cached system prompt (static)                             │
│  - Minimal context (dynamic)                                 │
│  - Constrained output instructions                           │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────────┐
│              Response Processor                              │
│  - Extract code edits                                        │
│  - Apply changes locally                                     │
│  - Update index incrementally                                │
└──────────────────────────────────────────────────────────────┘
```

### Core Modules

```rust
// 1. Codebase indexer
mod indexer {
    struct CodebaseIndex {
        symbols: SymbolTable,           // Fast lookup
        dependencies: DependencyGraph,   // Context resolution
        embeddings: VectorStore,         // Semantic search
    }
}

// 2. Context builder
mod context {
    struct ContextBuilder {
        max_tokens: usize,              // e.g., 8000
        max_files: usize,               // e.g., 10
        cache: ConversationCache,
    }
    
    fn build_minimal_context(
        query: &str,
        index: &CodebaseIndex,
    ) -> MinimalContext { }
}

// 3. Model router
mod router {
    enum Model { Haiku4, Sonnet45 }
    
    fn select_model(task_type: TaskType) -> Model { }
    fn estimate_cost(context: &Context, model: Model) -> f64 { }
}

// 4. API client with optimization
mod api {
    struct OptimizedClient {
        static_cache: Vec<Message>,     // System prompt, rules
        conversation_state: HashMap,     // Track sent context
    }
}
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1)
**Goal**: Reduce cache operations by 60%

- [ ] Implement codebase indexer (AST + symbol table)
- [ ] Build dependency graph analyzer
- [ ] Create minimal context builder
- [ ] Add local grep/search (avoid AI for lookups)

**Expected Savings**: $8-10/day ($240-300/month)

### Phase 2: Output Optimization (Week 2)
**Goal**: Reduce output tokens by 35%

- [ ] Implement constrained prompting system
- [ ] Add diff-based editing (send only changes)
- [ ] Create response templates for common tasks
- [ ] Add output token tracking & warnings

**Expected Savings**: Additional $0.60/day ($18/month)

### Phase 3: Multi-Model Routing (Week 3)
**Goal**: Route 70% of tasks to Haiku

- [ ] Implement task classifier
- [ ] Build Haiku/Sonnet router
- [ ] Add escalation mechanism (Haiku → Sonnet if needed)
- [ ] Create cost tracking dashboard

**Expected Savings**: Additional $6-8/day ($180-240/month)

### Phase 4: Advanced Optimization (Week 4)
**Goal**: Fine-tune for maximum efficiency

- [ ] Implement semantic search with embeddings
- [ ] Add conversation-aware caching
- [ ] Build incremental index updates
- [ ] Create cost analytics & reporting

**Expected Savings**: Additional $2-3/day ($60-90/month)

### Phase 5: Local + Multi-Cloud Hybrid (Week 5)
**Goal**: Achieve 95-98% cost reduction

- [ ] Implement local model manager
- [ ] Integrate Groq API (free tier)
- [ ] Integrate Gemini Flash API (4x cheaper than Haiku)
- [ ] Implement multi-tier routing

**Expected Savings**: Additional $10-12/day ($300-360/month)

---

## Cost Projections

### Current State
```
Daily:   $15.20
Monthly: $456.00
Yearly:  $5,472.00
```

### After Phase 1 (Context Optimization)
```
Daily:   $5.20 - $7.20 (66% reduction)
Monthly: $156 - $216
Yearly:  $1,872 - $2,592
Savings: $240-$300/month
```

### After Phase 2 (+ Output Optimization)
```
Daily:   $4.60 - $6.60 (70% reduction)
Monthly: $138 - $198
Yearly:  $1,656 - $2,376
Savings: $258-$318/month
```

### After Phase 3 (+ Multi-Model)
```
Daily:   $2.20 - $3.20 (79-86% reduction)
Monthly: $66 - $96
Yearly:  $792 - $1,152
Savings: $360-$390/month
```

### After Phase 4 (Full Optimization)
```
Daily:   $1.50 - $2.50 (84-90% reduction)
Monthly: $45 - $75
Yearly:  $540 - $900
Savings: $381-$411/month ($4,572-$4,932/year)
```

### After Phase 5 (Local + Multi-Cloud Hybrid) ⭐ RECOMMENDED
```
Daily:   $0.15 - $0.60 (96-99% reduction)
Monthly: $4.50 - $18.00
Yearly:  $54 - $216
Savings: $438-$451/month ($5,256-$5,418/year)
```

**Breakdown** (Hybrid approach):
- 70% tasks: Local models ($0/day)
- 15% tasks: Groq free tier ($0/day)
- 10% tasks: Gemini Flash ($0.07/day)
- 5% tasks: Sonnet 4.5 ($0.50/day)
- **Total: ~$0.57/day average**

**Additional Benefits**:
- Response time: 10-200ms (vs 2-5s)
- Works offline for 70% of tasks
- No rate limits on local inference
- Privacy: sensitive code stays local

---

## Key Performance Indicators (KPIs)

### Cost Metrics
- [ ] **Daily API cost < $3.00** (80% reduction target)
- [ ] **Cache reads < 5M tokens/day** (84% reduction)
- [ ] **Output tokens < 35K/day** (54% reduction)
- [ ] **Average cost per request < $0.15**

### Efficiency Metrics
- [ ] **Context size < 20KB per request** (vs full codebase)
- [ ] **70%+ requests use Haiku** (cost optimization)
- [ ] **90%+ cache hit rate** (for static context)
- [ ] **Response time < 3s** (faster + cheaper)

### Accuracy Metrics
- [ ] **Success rate > 95%** (correct answers)
- [ ] **Escalation rate < 15%** (Haiku → Sonnet)
- [ ] **Re-query rate < 10%** (context sufficient first time)

---

## Risk Mitigation

### Potential Issues

**Risk 1: Reduced Context = Lower Accuracy**
- *Mitigation*: Implement confidence scoring; escalate to full context if needed
- *Fallback*: Allow user to manually request more context
- *Monitoring*: Track success rate; adjust context window if < 90%

**Risk 2: Haiku Insufficient for Complex Tasks**
- *Mitigation*: Conservative routing; prefer Sonnet for ambiguity
- *Fallback*: Auto-escalate on Haiku failure/low confidence
- *Monitoring*: Track Haiku success rate by task type

**Risk 3: Index Maintenance Overhead**
- *Mitigation*: Incremental updates only; lazy indexing
- *Fallback*: Disable indexing for small projects (< 50 files)
- *Monitoring*: Track index build time vs API cost savings

**Risk 4: Over-Optimization Complexity**
- *Mitigation*: Phase-based rollout; measure each phase
- *Fallback*: Feature flags to disable optimizations
- *Monitoring*: Cost vs complexity ratio

---

## Success Criteria

### Phase 1 Success (Week 1)
- Daily cost reduced to < $7.00 (54% reduction)
- Context size < 30KB average
- No accuracy degradation (> 90% success rate)

### Phase 2 Success (Week 2)
- Daily cost reduced to < $6.00 (61% reduction)
- Output tokens < 50K/day
- Response quality maintained

### Phase 3 Success (Week 3)
- Daily cost reduced to < $3.50 (77% reduction)
- 60%+ tasks routed to Haiku
- Escalation rate < 20%

### Phase 4 Success (Week 4)
- **Daily cost reduced to < $2.50 (84% reduction)**
- **Monthly savings: $380+ ($4,560/year)**
- **All KPIs met**
- **Production-ready tool**

### Phase 5 Success (Week 5)
- **Daily cost reduced to < $0.60 (96% reduction)**
- **Monthly savings: $438+ ($5,256/year)**
- **All KPIs met**
- **Production-ready tool**

---

## Alternative Approaches (Considered)

### 1. Fine-Tuned Smaller Model
- **Pros**: Potentially much cheaper (10x+ reduction)
- **Cons**: Training cost, maintenance, less flexible
- **Verdict**: Revisit if costs don't improve enough

### 2. Hybrid Local LLM + Cloud
- **Pros**: Free inference for simple tasks
- **Cons**: GPU costs, limited quality, complexity
- **Verdict**: Not worth complexity for current scale

### 3. Batch Processing
- **Pros**: Potential volume discounts
- **Cons**: Poor UX (delays), not offered by Anthropic
- **Verdict**: Not applicable

### 4. Competitor APIs (OpenAI, Gemini)
- **Pros**: Different pricing models
- **Cons**: Quality differences, migration cost
- **Verdict**: Monitor pricing, but Claude quality preferred

---

## Next Steps

1. **Immediate**: Review and approve this plan
2. **Week 1**: Implement Phase 1 (context optimization)
3. **Week 2**: Measure Phase 1 results, implement Phase 2
4. **Week 3**: Implement multi-model routing
5. **Week 4**: Fine-tune and productionize
6. **Week 5**: Implement local + multi-cloud hybrid

**Target Launch**: Full optimization deployed in 5 weeks
**Target Savings**: $438+/month (96% reduction)
**Break-even**: Immediate (no infrastructure costs)

---

## Appendix: Code Snippets

### A. Minimal Context Builder

```rust
use std::collections::HashSet;

struct ContextBuilder {
    max_tokens: usize,
    max_files: usize,
}

impl ContextBuilder {
    fn build_context(&self, query: &str, index: &CodebaseIndex) -> String {
        let relevant_symbols = self.extract_symbols(query);
        let mut files = HashSet::new();
        let mut context = String::new();
        
        // Get relevant files
        for symbol in relevant_symbols {
            if let Some(file) = index.find_definition(&symbol) {
                files.insert(file);
                if files.len() >= self.max_files {
                    break;
                }
            }
        }
        
        // Build minimal context
        for file in files {
            context.push_str(&format!("// File: {}\n", file));
            context.push_str(&index.get_file_content(file));
            context.push_str("\n\n");
            
            if context.len() > self.max_tokens * 4 {
                break;
            }
        }
        
        context
    }
    
    fn extract_symbols(&self, query: &str) -> Vec<String> {
        // Simple regex-based extraction
        // In production: use NLP/embeddings
        query.split_whitespace()
            .filter(|w| w.chars().next().unwrap_or('_').is_alphanumeric())
            .map(String::from)
            .collect()
    }
}
```

### B. Model Router

```rust
use anthropic::{Model, Client};

enum TaskComplexity {
    Simple,   // Haiku
    Medium,   // Haiku with Sonnet fallback
    Complex,  // Sonnet
}

struct ModelRouter {
    haiku_client: Client,
    sonnet_client: Client,
}

impl ModelRouter {
    fn classify_task(&self, query: &str) -> TaskComplexity {
        let complex_keywords = [
            "design", "architecture", "refactor", "explain why",
            "complex", "optimize", "performance"
        ];
        
        let simple_keywords = [
            "find", "search", "show", "view", "list",
            "what is", "where is"
        ];
        
        if complex_keywords.iter().any(|k| query.to_lowercase().contains(k)) {
            TaskComplexity::Complex
        } else if simple_keywords.iter().any(|k| query.to_lowercase().contains(k)) {
            TaskComplexity::Simple
        } else {
            // Default to Haiku, escalate if needed
            TaskComplexity::Medium
        }
    }
    
    async fn route_request(&self, query: &str, context: &str) -> Result<String> {
        let complexity = self.classify_task(query);
        
        match complexity {
            TaskComplexity::Simple => {
                self.haiku_client.complete(query, context).await
            }
            TaskComplexity::Medium => {
                let result = self.haiku_client.complete(query, context).await;
                if self.is_low_confidence(&result?) {
                    // Escalate to Sonnet
                    self.sonnet_client.complete(query, context).await
                } else {
                    result
                }
            }
            TaskComplexity::Complex => {
                self.sonnet_client.complete(query, context).await
            }
        }
    }
    
    fn is_low_confidence(&self, response: &str) -> bool {
        response.contains("I'm not sure") 
            || response.contains("unclear")
            || response.len() < 50
    }
}
```

### C. Cost Tracking

```rust
use std::sync::atomic::{AtomicU64, Ordering};

struct CostTracker {
    input_tokens: AtomicU64,
    output_tokens: AtomicU64,
    cache_read_tokens: AtomicU64,
    cache_write_tokens: AtomicU64,
}

impl CostTracker {
    fn record_request(&self, usage: &TokenUsage) {
        self.input_tokens.fetch_add(usage.input, Ordering::Relaxed);
        self.output_tokens.fetch_add(usage.output, Ordering::Relaxed);
        self.cache_read_tokens.fetch_add(usage.cache_read, Ordering::Relaxed);
        self.cache_write_tokens.fetch_add(usage.cache_write, Ordering::Relaxed);
    }
    
    fn calculate_cost(&self, model: Model) -> f64 {
        let (input_cost, output_cost, cache_read_cost, cache_write_cost) = 
            match model {
                Model::Sonnet45 => (3.0, 15.0, 0.3, 3.75),
                Model::Haiku4 => (0.8, 4.0, 0.08, 1.0),
            };
        
        let input = self.input_tokens.load(Ordering::Relaxed) as f64;
        let output = self.output_tokens.load(Ordering::Relaxed) as f64;
        let cache_read = self.cache_read_tokens.load(Ordering::Relaxed) as f64;
        let cache_write = self.cache_write_tokens.load(Ordering::Relaxed) as f64;
        
        (input * input_cost + output * output_cost + 
         cache_read * cache_read_cost + cache_write * cache_write_cost) / 1_000_000.0
    }
    
    fn daily_report(&self) {
        println!("Daily Cost Report");
        println!("Input: {} tokens", self.input_tokens.load(Ordering::Relaxed));
        println!("Output: {} tokens", self.output_tokens.load(Ordering::Relaxed));
        println!("Cache Read: {} tokens", self.cache_read_tokens.load(Ordering::Relaxed));
        println!("Cache Write: {} tokens", self.cache_write_tokens.load(Ordering::Relaxed));
        println!("Total Cost: ${:.2}", self.calculate_cost(Model::Sonnet45));
    }
}
```

---

**End of Plan**

*Estimated completion: 5 weeks*  
*Projected savings: $438+/month (96% reduction)*  
*ROI: Immediate (development time only)*
