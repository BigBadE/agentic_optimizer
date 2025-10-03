# Context Fetching Benchmarks

Benchmarks for evaluating context retrieval quality using hybrid BM25 + vector search.

## Current Performance (Phase 5 - Graph Ranking)

| Metric | Current | Target (Industry) | Gap | Status |
|--------|---------|------------------|-----|--------|
| **Precision@3** | 30.0% | 60% | +30% | 🔴 50% of target |
| **Precision@10** | 20.0% | 55% | +35% | 🔴 36% of target |
| **Recall@10** | 49.4% | 70% | +20.6% | 🟡 71% of target |
| **MRR** | 0.440 | 0.700 | +0.260 | 🔴 63% of target |
| **NDCG@10** | 0.437 | 0.750 | +0.313 | 🔴 58% of target |
| **Critical in Top-3** | 25.0% | 65% | +40% | 🔴 38% of target |

**Test Suite**: 20 diverse test cases covering real-world coding agent scenarios

*Targets set to estimated Cursor/GitHub Copilot performance levels. See `SOLVING_DOC_DOMINANCE.md` for fundamental solutions.*

## Quick Start

```bash
# Run all benchmarks
cargo run --bin benchmark

# Generate report
cargo run --bin benchmark --report benchmarks/results.md

# Run specific test
cargo run --bin benchmark --test css_parsing
```

## Structure

```
benchmarks/
├── README.md                    # This file
├── SETUP.md                     # Setup and configuration
├── test_cases/                  # Test case definitions
│   └── valor/                   # 20 test cases for Valor browser
└── test_repositories/           # External projects
    └── valor/                   # Valor browser engine
```

## Test Categories

**20 Test Cases** covering:
- **Core Rendering** (4): CSS parsing, DOM tree, layout engine, rendering pipeline
- **JavaScript Integration** (4): Runtime, console, modules, event delegation
- **CSS Advanced** (4): Flexbox, Grid, animations, viewport units
- **Performance** (3): Selector matching, DOM mutations, async rendering
- **Debugging** (5): HTML errors, z-index, GPU text, box model, fetch API

## Test Case Format

Each test case is a TOML file:

```toml
name = "CSS Parsing Implementation"
query = "how does CSS parsing work"
project_root = "benchmarks/test_repositories/valor"

[[expected]]
path = "crates/css/src/parser.rs"
priority = "critical"  # critical, high, medium, low
reason = "Main CSS parser implementation"

[[excluded]]
path = "crates/renderer"
reason = "Rendering, not parsing"
```

See `SETUP.md` for detailed format documentation.

## Metrics Explained

- **Precision@3**: % of top-3 results that are relevant (measures accuracy)
- **Recall@10**: % of relevant files found in top-10 (measures coverage)
- **MRR**: Mean Reciprocal Rank - average of 1/rank for first relevant result
- **NDCG@10**: Normalized Discounted Cumulative Gain - quality-weighted ranking
- **Critical in Top-3**: % of critical files appearing in top-3 results

## Implementation Progress

### ✅ Phase 1-2: Base Improvements (0% → 25% P@3)
- Enhanced BM25 tokenization with n-grams
- File type boosting (code 1.7x, docs 0.1x)
- Path-based boosting (/src/ 1.3x, /docs/ 0.5x)
- Test file filtering (0.1x penalty)
- Weighted score combination (replaced RRF)
- Query intent detection ("how"/"implement"/"fix")
- BM25 threshold filtering (0.75)

### ✅ Phase 4: Pattern Boost (25% → 25% P@3, 47.9% → 50.4% R@10)
- Pattern-based importance boost:
  - impl + struct: 1.3x
  - Trait definitions: 1.2x
  - Public API-rich: 1.2x
  - Module docs: 1.15x

### ✅ Phase 5: Graph Ranking (25% → 30% P@3, 20% → 25% Critical@3)
- Import graph analysis using rust-analyzer:
  - Boost heavily-imported files (1.3x if >5 importers)
  - Boost orchestrator files (1.2x if >10 imports)
  - Uses rust-analyzer backend for accurate import resolution
  - Graceful fallback if rust-analyzer unavailable

### 🔄 To Hit Industry Targets (60% P@3, 70% R@10)

**The Core Problem**: Generic text embeddings favor documentation over code.

**Fundamental Solutions** (see `SOLVING_DOC_DOMINANCE.md` for details):

1. **LLM Re-ranking** 🥇 RECOMMENDED
   - Use Gemini Flash to re-rank top 50 candidates
   - **Impact**: 30% → 50-60% P@3 (+20-30%)
   - **Cost**: ~$0.004/query
   - **Effort**: 1 week

2. **Code-Aware Embeddings** 🥈
   - Replace generic embeddings with CodeBERT/GraphCodeBERT
   - **Impact**: +15-20% P@3
   - **Cost**: One-time re-embedding
   - **Effort**: 2-3 weeks

3. **Custom Fine-Tuned Model** 🥉
   - Train on "code > docs" preference
   - **Impact**: 60%+ P@3 (industry level)
   - **Cost**: $50-200 training
   - **Effort**: 1-2 months

**Recommended Path**: Implement LLM re-ranking (Phase 6) → Add CodeBERT (Phase 7) → Hit all targets

## Example Output

```
═══════════════════════════════════════════════════════════
Running: CSS Parsing Implementation
═══════════════════════════════════════════════════════════

⚙️  Initializing vector search...
  Hybrid search: 1811 embeddings, 1811 BM25 docs
  Combined 50 results using weighted scores
  After filtering: 25 results

# Benchmark: CSS Parsing Implementation

**Query**: "how does CSS parsing work"

## Metrics
- **Precision@3**:  66.7%
- **Recall@10**:    85.7%
- **MRR**:          0.500
- **NDCG@10**:      0.765
- **Critical in Top-3**: 100.0%

## Top 10 Results
1. ✅ crates/css/src/parser.rs (expected: Critical)
2. ✅ crates/css/src/tokenizer.rs (expected: Critical)
3. ❌ crates/css/src/lib.rs (not expected)
4. ✅ crates/css/orchestrator/src/lib.rs (expected: High)
...

═══════════════════════════════════════════════════════════
SUMMARY (20 test cases)
═══════════════════════════════════════════════════════════

Average Metrics:
  Precision@3:        25.0%
  Recall@10:          50.4%
  MRR:                0.469
  Critical in Top-3:  20.0%
```

## Files

- **SETUP.md** - Detailed setup and test case format
- **test_cases/valor/** - 20 test case definitions
- **phase4_step2_pattern_boost.md** - Latest benchmark results
