# Model Configuration Guide

## Environment Variables

The system now supports configurable models via environment variables:

```bash
# Small, fast model for simple tasks (default: qwen2.5-coder:1.5b)
export LOCAL_SMALL_MODEL="qwen2.5-coder:1.5b-instruct-q4_K_M"

# Medium model for context planning and analysis (default: qwen2.5-coder:7b)
export LOCAL_MEDIUM_MODEL="qwen2.5-coder:7b-instruct-q4_K_M"

# Large model for complex reasoning (default: qwen2.5-coder:32b)
export LARGE_MODEL="qwen2.5-coder:32b"

# Embedding model for semantic search (default: nomic-embed-text)
export EMBEDDING_MODEL="nomic-embed-text"
```

### Windows PowerShell
```powershell
$env:LOCAL_SMALL_MODEL = "qwen2.5-coder:1.5b"
$env:MEDIUM_MODEL = "qwen2.5-coder:7b"
$env:LARGE_MODEL = "qwen2.5-coder:32b"
$env:EMBEDDING_MODEL = "nomic-embed-text"
```

### Windows CMD
```cmd
set LOCAL_SMALL_MODEL=qwen2.5-coder:1.5b
set MEDIUM_MODEL=qwen2.5-coder:7b
set LARGE_MODEL=qwen2.5-coder:32b
set EMBEDDING_MODEL=nomic-embed-text
```

## Using Qwen with CUDA GPU

### Prerequisites
- NVIDIA GPU with CUDA support
- CUDA Toolkit installed (11.8+ or 12.x)
- Ollama installed

### Setup Steps

1. **Verify CUDA is available**:
   ```bash
   nvidia-smi
   ```
   You should see your GPU listed with driver version and CUDA version.

2. **Pull Qwen models with Ollama**:
   ```bash
   # Small model (1.5B parameters) - fast, ~1GB VRAM
   ollama pull qwen2.5-coder:1.5b
   
   # Medium model (7B parameters) - balanced, ~4GB VRAM
   ollama pull qwen2.5-coder:7b
   
   # Large model (32B parameters) - powerful, ~18GB VRAM
   ollama pull qwen2.5-coder:32b
   
   # Embedding model for semantic search
   ollama pull nomic-embed-text
   ```

3. **Ollama automatically uses CUDA**:
   - Ollama detects CUDA and uses GPU acceleration automatically
   - No additional configuration needed
   - Check GPU usage: `nvidia-smi` while running inference

4. **Verify GPU usage**:
   ```bash
   # Start Ollama server
   ollama serve
   
   # In another terminal, run a test
   ollama run qwen2.5-coder:7b "Write a hello world in Rust"
   
   # Watch GPU usage in third terminal
   watch -n 1 nvidia-smi
   ```

### Expected VRAM Usage

| Model | Parameters | VRAM (4-bit) | VRAM (8-bit) | Speed |
|-------|-----------|--------------|--------------|-------|
| qwen2.5-coder:1.5b | 1.5B | ~1GB | ~2GB | Very Fast |
| qwen2.5-coder:7b | 7B | ~4GB | ~8GB | Fast |
| qwen2.5-coder:32b | 32B | ~18GB | ~36GB | Slower |

### Troubleshooting

**Ollama not using GPU:**
- Check `ollama serve` output - should mention GPU
- Verify CUDA: `nvidia-smi`
- Reinstall Ollama if needed

**Out of Memory:**
- Use smaller model (1.5b or 7b)
- Check VRAM: `nvidia-smi`
- Close other GPU applications

**Slow inference:**
- Check if CPU fallback: `ollama ps` shows device
- Verify GPU load: `nvidia-smi`
- Consider smaller model for faster responses

## Model Selection Strategy

The codebase automatically selects models based on task complexity:

- **Simple tasks** → `LOCAL_SMALL_MODEL` (1.5b)
  - Quick classifications
  - Basic parsing
  - Fast iterations
  
- **Medium tasks** → `MEDIUM_MODEL` (7b) 
  - Context planning (current subagent)
  - Code analysis
  - Default for most tasks
  
- **Complex tasks** → `LARGE_MODEL` (32b)
  - Architecture decisions
  - Deep reasoning
  - Multi-step planning

## Semantic Search (Future)

For queries like "Fix the infinite loading bug", semantic search would:

1. **Embed all source files** using `nomic-embed-text`
2. **Embed user query** into vector space
3. **Find similar code** via cosine similarity
4. **Return top-k files** most relevant to the concept

**Benefits:**
- Finds conceptually related code without exact keywords
- Great for vague/exploratory queries
- Discovers cross-cutting concerns

**Cost:**
- Initial embedding: ~1-2 seconds per file (one-time)
- Query embedding: ~10-50ms
- Storage: ~4KB per file (vectors)

**Recommendation:**
- Implement if you frequently make conceptual queries
- Use Ollama's `/api/embeddings` endpoint (already running)
- Cache embeddings with file modification timestamps
