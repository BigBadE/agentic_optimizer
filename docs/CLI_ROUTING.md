# CLI Routing Integration

The `merlin` CLI now includes multi-model routing capabilities through the `route` command.

## Usage

```bash
merlin route "your request here" [OPTIONS]
```

### Options

- `-p, --project <PATH>` - Project root directory (default: current directory)
- `--local` - Use only local models (Ollama), disable remote tiers
- `--no-validate` - Disable validation pipeline (enabled by default)
- `--verbose` - Show detailed routing decisions and task breakdown
- `--no-tui` - Disable TUI mode (plain terminal output)

## Examples

### Basic Request
```bash
merlin route "Add error handling to the parser"
```

Output:
```
=== Multi-Model Routing ===
Request: Add error handling to the parser
Project: .

Initializing orchestrator...
Analyzing request...
✓ Analysis complete: 1 task(s) generated

Executing tasks...
✓ Completed: 1 task(s) in 0.52s

Results:
  1. Task TaskId(...)
     Tier: Local(qwen2.5-coder:7b)
     Duration: 520ms
     Tokens: 1234

Summary:
  Total tokens: 1234
  Total duration: 520ms
  Average per task: 520ms
```

### Complex Refactor with Validation
```bash
merlin route "Refactor the parser module" --verbose
```

Output:
```
=== Multi-Model Routing ===
Request: Refactor the parser module
Project: .

Configuration:
  Local enabled: true
  Groq enabled: true
  Validation enabled: true
  Max concurrent: 4

Initializing orchestrator...
Analyzing request...
✓ Analysis complete: 3 task(s) generated

Tasks:
  1. Analyze current structure: Refactor the parser module (complexity: Medium, priority: Medium)
  2. Refactor: Refactor the parser module (complexity: Complex, priority: Medium)
     Dependencies: 1 task(s)
  3. Test refactored code: Refactor the parser module (complexity: Medium, priority: Medium)
     Dependencies: 1 task(s)

Execution strategy: Pipeline

Executing tasks...
✓ Completed: 3 task(s) in 2.34s

Results:
  1. Task TaskId(...)
     Tier: Groq(llama-3.1-70b-versatile)
     Duration: 650ms
     Tokens: 2456
     Validation: ✓ PASSED (score: 1.00)
     Response preview: The parser module can be refactored by...

  2. Task TaskId(...)
     Tier: Premium(openrouter/deepseek-coder)
     Duration: 1200ms
     Tokens: 4567
     Validation: ✓ PASSED (score: 0.95)
     Response preview: Here's the refactored parser implementation...

  3. Task TaskId(...)
     Tier: Local(qwen2.5-coder:7b)
     Duration: 490ms
     Tokens: 1890
     Validation: ✓ PASSED (score: 1.00)
     Response preview: Added comprehensive tests for the refactored...

Summary:
  Total tokens: 8913
  Total duration: 2340ms
  Average per task: 780ms
```

### Multi-file Modification
```bash
merlin route "Add logging to main.rs and utils.rs" --no-tui --verbose
```

### Local-Only Mode (Zero Cost)
```bash
merlin route "Add comments to all public functions" --local
```

This restricts routing to only use Ollama models, ensuring zero API costs. Perfect for:
- Quick iterations and experiments
- Working offline
- Cost-sensitive workflows
- Learning and testing

## How It Works

### 1. Request Analysis
The orchestrator analyzes your request using the `LocalTaskAnalyzer`:
- **Intent extraction**: Identifies action (create, modify, refactor, etc.)
- **Complexity estimation**: Evaluates task difficulty
- **Task decomposition**: Breaks complex requests into subtasks
- **Dependency tracking**: Builds execution graph

### 2. Model Routing
Each task is routed to the appropriate tier using strategies:
- **Quality Critical** (Priority 100): Critical/High priority → Premium models
- **Long Context** (Priority 90): Large context → Appropriate tier
- **Cost Optimization** (Priority 70): Non-critical → Free tiers
- **Complexity Based** (Priority 50): Fallback based on complexity

### 3. Parallel Execution
Tasks are executed based on their dependencies:
- **Sequential**: Single task or strict dependencies
- **Parallel**: Independent tasks run concurrently
- **Pipeline**: Tasks with dependency chains

### 4. Validation (Enabled by Default)
Every response passes through the validation pipeline unless `--no-validate` is supplied:
1. **Syntax Check**: Heuristic-based validation (0ms)
2. **Build Check**: Isolated `cargo check` (~5-30s)
3. **Test Check**: Isolated `cargo test` (~10-300s)
4. **Lint Check**: Clippy validation (~5-30s)

## Model Tiers

### Local (Free)
- **Provider**: Ollama
- **Models**: Qwen 2.5 Coder 7B, DeepSeek Coder 6.7B
- **Cost**: $0
- **Latency**: ~100ms
- **Requirements**: Ollama installed and running

### Groq (Free)
- **Provider**: Groq
- **Models**: Llama 3.1 70B Versatile
- **Cost**: $0 (with rate limits)
- **Latency**: ~500ms
- **Requirements**: `GROQ_API_KEY` environment variable

### Premium (Paid)
- **Providers**: OpenRouter, Anthropic
- **Models**: Claude 3.5 Sonnet, Claude 3 Haiku, DeepSeek Coder
- **Cost**: $0.0000002 - $0.000015 per token
- **Latency**: ~2000ms
- **Requirements**: `OPENROUTER_API_KEY` or `ANTHROPIC_API_KEY`

## Configuration

The routing system uses default configuration, but you can customize it by modifying the code or adding a config file (future feature).

Default settings:
- Local enabled: `true`
- Groq enabled: `true`
- Premium enabled: `true`
- Max concurrent tasks: `4`
- Validation enabled: `true`
- Early exit on validation failure: `true`

## Comparison with Other Commands

### `merlin query` vs `merlin route`

**`merlin query`**:
- Direct provider selection
- No task decomposition
- No validation pipeline
- Simpler, faster for straightforward queries

**`merlin route`**:
- Multi-model routing
- Automatic tier selection
- Smart task decomposition
- Validation pipeline (enabled by default)
- Better for complex, multi-step requests

## Tips

1. **Use `--verbose`** for the first few requests to understand how tasks are decomposed
{{ ... }}
3. **Start simple** - try basic requests before complex refactors
4. **Check Ollama** - Ensure Ollama is running for local tier access
5. **Set API keys** - Configure `GROQ_API_KEY` for free tier access to larger models

## Troubleshooting

### "Ollama not available"
- Ensure Ollama is installed and running: `ollama serve`
- Check if models are pulled: `ollama list`
- Pull recommended model: `ollama pull qwen2.5-coder:7b`

### "No available tier"
- Check API keys are set in environment
- Verify network connectivity
- Try with `--verbose` to see routing decisions

### Validation failures
- Check that your project builds: `cargo check`
- Ensure tests pass: `cargo test`
- Run clippy: `cargo clippy`

## Future Enhancements

- Configuration file support (TOML/JSON)
- Interactive TUI mode
- Response caching
- Metrics tracking
- Custom routing strategies
- Streaming responses
- Multi-turn conversations

## See Also

- [Routing Architecture](ROUTING_ARCHITECTURE.md) - Complete architecture documentation
- [merlin-routing README](../crates/merlin-routing/README.md) - Library documentation
- [Integration Tests](../crates/merlin-routing/tests/integration_tests.rs) - Test scenarios

