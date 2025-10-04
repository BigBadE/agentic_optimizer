# Design Overview - Merlin

## Vision

Build a cost-optimized AI coding agent that reduces API costs by 96% while maintaining high quality through intelligent routing, local models, and minimal context strategies.

## Core Principles

1. **Modularity**: All components are decoupled and swappable
2. **Progressive Enhancement**: Start with MVP, add optimizations in phases
3. **Testability**: Every component is independently testable
4. **Observability**: Track costs, performance, and quality metrics
5. **Fail-Safe**: Always escalate on uncertainty rather than produce wrong answers

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          User Interface                          │
│                     (CLI / LSP / API Server)                     │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                       Request Handler                            │
│  - Parse user query                                              │
│  - Manage conversation state                                     │
│  - Coordinate responses                                          │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                      Routing Engine                              │
│  - Analyze query complexity                                      │
│  - Select appropriate model tier                                 │
│  - Determine required context                                    │
└─────────┬────────────────────────────────────┬──────────────────┘
          │                                    │
    ┌─────▼─────┐                      ┌──────▼──────┐
    │  Context  │                      │   Model     │
    │  Builder  │                      │   Router    │
    └─────┬─────┘                      └──────┬──────┘
          │                                    │
    ┌─────▼──────────────────┐        ┌───────▼──────────────┐
    │   Codebase Index       │        │  Model Providers     │
    │   - Symbol table       │        │  - Local (Ollama)    │
    │   - Dependency graph   │        │  - Groq (free)       │
    │   - File metadata      │        │  - Gemini Flash      │
    │   - Embeddings (opt)   │        │  - Sonnet (premium)  │
    └────────────────────────┘        └──────────────────────┘
                                                │
                             ┌──────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                      Response Processor                          │
│  - Parse model output                                            │
│  - Extract tool calls / edits                                    │
│  - Apply changes                                                 │
│  - Update metrics                                                │
└────────────────────────────┬────────────────────────────────────┘
                             │
┌────────────────────────────▼────────────────────────────────────┐
│                     Observability Layer                          │
│  - Cost tracking                                                 │
│  - Performance metrics                                           │
│  - Quality scoring                                               │
│  - Error logging                                                 │
└──────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

### Request Handler
- **Input**: User query + conversation history
- **Output**: Final response to user
- **Responsibilities**:
  - Parse and validate user input
  - Maintain conversation context
  - Coordinate between routing engine and response processor
  - Handle errors and retries

### Routing Engine
- **Input**: Parsed query + context metadata
- **Output**: Routing decision (model tier + context strategy)
- **Responsibilities**:
  - Classify query complexity (simple/medium/complex)
  - Determine minimum required context
  - Select optimal model tier based on cost/quality tradeoff
  - Implement escalation logic

### Context Builder
- **Input**: Query + routing decision
- **Output**: Minimal context package for model
- **Responsibilities**:
  - Query codebase index for relevant files
  - Resolve dependencies
  - Build minimal context within token budget
  - Cache static context for reuse

### Codebase Index
- **Input**: File system events, explicit refresh requests
- **Output**: Queryable index of codebase
- **Responsibilities**:
  - Build symbol table (functions, structs, modules)
  - Create dependency graph
  - Generate file metadata (size, last modified, etc.)
  - Optional: semantic embeddings for similarity search
  - Incremental updates on file changes

### Model Router
- **Input**: Query + context + selected tier
- **Output**: Model response
- **Responsibilities**:
  - Manage connections to multiple model providers
  - Implement retry logic with backoff
  - Handle API rate limits
  - Track token usage per provider
  - Auto-escalate on low confidence or errors

### Model Providers (Adapters)
- **Local Provider**: Ollama integration for local models
- **Groq Provider**: Free tier API for fast inference
- **Gemini Provider**: Flash 2.0 for cost-effective quality
- **Sonnet Provider**: Claude Sonnet 4.5 for premium quality
- **Interface**: All implement same `ModelProvider` trait

### Response Processor
- **Input**: Raw model output
- **Output**: Structured response + actions
- **Responsibilities**:
  - Parse model output (tool calls, code edits, etc.)
  - Validate responses
  - Apply file edits
  - Update codebase index if files changed
  - Extract metrics (confidence, token usage)

### Observability Layer
- **Responsibilities**:
  - Track costs per provider, per day, per query type
  - Monitor response times and success rates
  - Score response quality (when possible)
  - Generate reports and alerts
  - Persist metrics for analysis

## Data Flow

### Simple Query (e.g., "Find function foo")
```
User Query
  → Request Handler (parse)
  → Routing Engine (classify: simple)
  → Context Builder (query local index)
  → [DONE] Return results (no API call)
  → Cost: $0, Time: ~10ms
```

### Medium Query (e.g., "Refactor function bar")
```
User Query
  → Request Handler
  → Routing Engine (classify: medium)
  → Context Builder (get bar + dependencies)
  → Model Router (try local 7B first)
  → Local Provider (Qwen2.5-Coder-7B)
  → Response Processor (apply edits)
  → Cost: $0, Time: ~200ms
```

### Complex Query (e.g., "Design new auth system")
```
User Query
  → Request Handler
  → Routing Engine (classify: complex)
  → Context Builder (get auth-related files)
  → Model Router (try Groq free tier)
  → Groq Provider (Llama 3.1 70B)
  → [If low confidence] Escalate to Gemini Flash
  → [If still uncertain] Escalate to Sonnet
  → Response Processor
  → Cost: $0-0.50, Time: 1-3s
```

## Configuration System

All behavior is configurable via `config.toml`:

```toml
[routing]
enable_local = true
enable_groq = true
enable_gemini = true
enable_sonnet = true

[routing.thresholds]
simple_max_tokens = 1000
medium_max_tokens = 5000
confidence_threshold = 0.85

[context]
max_files = 10
max_tokens = 8000
include_dependencies = true
include_tests = false

[local_models]
router_model = "phi3:mini"
coder_model = "qwen2.5-coder:7b"
device = "cuda"  # or "cpu"

[api_keys]
groq = "${GROQ_API_KEY}"
gemini = "${GEMINI_API_KEY}"
anthropic = "${ANTHROPIC_API_KEY}"

[observability]
enable_metrics = true
enable_logging = true
log_level = "info"
metrics_file = "metrics.json"
```

## Error Handling Strategy

### Principle: Escalate, Don't Hallucinate

```rust
enum ResponseConfidence {
    High,      // 0.9+ - return result
    Medium,    // 0.7-0.9 - escalate to next tier
    Low,       // <0.7 - escalate or fail
}

fn handle_response(response: ModelResponse) -> Result<Response> {
    match response.confidence {
        ResponseConfidence::High => Ok(response),
        ResponseConfidence::Medium => {
            if can_escalate() {
                escalate_to_next_tier()
            } else {
                Ok(response.with_disclaimer())
            }
        }
        ResponseConfidence::Low => {
            if can_escalate() {
                escalate_to_next_tier()
            } else {
                Err("Unable to provide confident answer")
            }
        }
    }
}
```

### Error Types
- **Retryable**: Network errors, rate limits → Retry with backoff
- **Escalatable**: Low confidence, parsing errors → Try next tier
- **Fatal**: Invalid input, missing API keys → Fail fast with clear message

## Testing Strategy

### Unit Tests
- Each module tested in isolation
- Mock external dependencies (API calls, file system)
- Test error conditions and edge cases

### Integration Tests
- Test full request flow with mocked APIs
- Verify routing decisions
- Validate context building
- Test escalation logic

### End-to-End Tests
- Use test projects with known structure
- Verify actual API calls (with test keys)
- Measure costs and performance
- Compare quality across models

### Benchmark Tests
- Track performance regressions
- Measure cost per query type
- Monitor quality scores
- Load testing for rate limit handling

## Deployment Options

### Option 1: CLI Tool
- Simple command-line interface
- Best for personal use
- Easy to integrate with existing workflow

### Option 2: LSP Server
- Integrate with any LSP-compatible editor
- Real-time code assistance
- More complex but better UX

### Option 3: API Server
- HTTP API for remote access
- Supports team use
- Requires authentication and security

**MVP starts with Option 1 (CLI)**

## Success Metrics

### Cost Metrics (Primary)
- **Daily API cost < $0.60** (96% reduction from $15.20)
- **Local inference coverage > 70%**
- **Free tier (Groq) coverage > 15%**
- **Premium tier (Sonnet) usage < 5%**

### Performance Metrics
- **P50 response time < 500ms** (local + Groq)
- **P95 response time < 3s** (including Gemini)
- **P99 response time < 5s** (including Sonnet)

### Quality Metrics
- **Success rate > 90%** (correct answer first try)
- **Escalation rate < 20%** (proportion requiring higher tier)
- **User satisfaction > 4/5** (subjective rating)

## Next Steps

See `ARCHITECTURE.md` for detailed module design and trait definitions.
See `PHASES.md` for phase-by-phase implementation plan.

