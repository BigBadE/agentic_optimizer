# Context Quality Benchmark Results

## Summary

**Test Cases**: 7

| Metric | Value | Target |
|--------|-------|--------|
| Precision@3 | 23.8% | 60% |
| Precision@10 | 12.7% | 55% |
| Recall@10 | 32.1% | 70% |
| MRR | 0.440 | 0.700 |
| NDCG@10 | 0.300 | 0.750 |
| Critical in Top-3 | 21.4% | 65% |

## Individual Test Results

### Graphics Backend

**Query**: "how does the graphics backend work with WGPU"

| Metric | Value |
|--------|-------|
| Precision@3 | 66.7% |
| Precision@10 | 30.0% |
| Recall@10 | 75.0% |
| MRR | 1.000 |
| NDCG@10 | 0.610 |
| Critical in Top-3 | 50.0% |

**Top 10 Results**:
1. `crates/renderer/src/backend.rs`
2. `crates/renderer/wgpu_backend/src/state.rs`
3. `crates/renderer/wgpu_backend/src/lib.rs`
4. `crates/renderer/wgpu_backend/src/state/initialization.rs`
5. `crates/renderer/wgpu_backend/src/state/opacity.rs`
6. `crates/renderer/src/lib.rs`
7. `crates/valor/src/test_support.rs`
8. `crates/renderer/wgpu_backend/src/state/rectangles.rs`
9. `crates/renderer/wgpu_backend/src/offscreen.rs`
10. `crates/renderer/wgpu_backend/src/state.rs`

<details>
<summary>Execution Logs</summary>

```
Running test: Graphics Backend
Query: how does the graphics backend work with WGPU
Project: benchmarks/test_repositories/valor
✓ Found 10 results
Retrieved files:
  1. crates/renderer/src/backend.rs
  2. crates/renderer/wgpu_backend/src/state.rs
  3. crates/renderer/wgpu_backend/src/lib.rs
  4. crates/renderer/wgpu_backend/src/state/initialization.rs
  5. crates/renderer/wgpu_backend/src/state/opacity.rs
  6. crates/renderer/src/lib.rs
  7. crates/valor/src/test_support.rs
  8. crates/renderer/wgpu_backend/src/state/rectangles.rs
  9. crates/renderer/wgpu_backend/src/offscreen.rs
  10. crates/renderer/wgpu_backend/src/state.rs
```
</details>

### Layout Engine

**Query**: "implement flexbox layout algorithm"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 10.0% |
| Recall@10 | 33.3% |
| MRR | 0.250 |
| NDCG@10 | 0.271 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
2. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
3. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
4. `crates/css/modules/flexbox/src/lib.rs`
5. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
6. `crates/css/modules/core/src/10_visual_details/part_10_6_3_height_of_blocks.rs`
7. `crates/css/orchestrator/src/style_model.rs`
8. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
9. `crates/css/modules/core/src/10_visual_details/part_10_6_3_height_of_blocks.rs`
10. `crates/css/modules/core/src/orchestrator/tree.rs`

<details>
<summary>Execution Logs</summary>

```
Running test: Layout Engine
Query: implement flexbox layout algorithm
Project: benchmarks/test_repositories/valor
✓ Found 15 results
Retrieved files:
  1. crates/css/modules/flexbox/src/8_single_line_layout/mod.rs
  2. crates/css/modules/flexbox/src/8_single_line_layout/mod.rs
  3. crates/css/modules/flexbox/src/8_single_line_layout/mod.rs
  4. crates/css/modules/flexbox/src/lib.rs
  5. crates/css/modules/flexbox/src/8_single_line_layout/mod.rs
  6. crates/css/modules/core/src/10_visual_details/part_10_6_3_height_of_blocks.rs
  7. crates/css/orchestrator/src/style_model.rs
  8. crates/css/modules/flexbox/src/8_single_line_layout/mod.rs
  9. crates/css/modules/core/src/10_visual_details/part_10_6_3_height_of_blocks.rs
  10. crates/css/modules/core/src/orchestrator/tree.rs
```
</details>

### Box Model Bug Fix

**Query**: "where is margin collapse calculated for adjacent elements"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 16.7% |
| Recall@10 | 33.3% |
| MRR | 1.000 |
| NDCG@10 | 0.630 |
| Critical in Top-3 | 100.0% |

**Top 10 Results**:
1. `crates/css/modules/core/src/8_box_model/part_8_3_1_collapsing_margins.rs`
2. `crates/css/modules/core/src/lib.rs`
3. `crates/css/modules/core/src/orchestrator/place_child.rs`
4. `crates/css/modules/core/src/8_box_model/spec.md`
5. `crates/css/modules/core/src/9_visual_formatting/spec.md`
6. `crates/css/modules/core/src/10_visual_details/spec.md`

<details>
<summary>Execution Logs</summary>

```
Running test: Box Model Bug Fix
Query: where is margin collapse calculated for adjacent elements
Project: benchmarks/test_repositories/valor
✓ Found 6 results
Retrieved files:
  1. crates/css/modules/core/src/8_box_model/part_8_3_1_collapsing_margins.rs
  2. crates/css/modules/core/src/lib.rs
  3. crates/css/modules/core/src/orchestrator/place_child.rs
  4. crates/css/modules/core/src/8_box_model/spec.md
  5. crates/css/modules/core/src/9_visual_formatting/spec.md
  6. crates/css/modules/core/src/10_visual_details/spec.md
```
</details>

### CSS Parsing Implementation

**Query**: "how does CSS parsing work"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 20.0% |
| Recall@10 | 50.0% |
| MRR | 0.333 |
| NDCG@10 | 0.327 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/html/src/lib.rs`
2. `crates/css/modules/values_units/src/chapter_5_percentages.rs`
3. `crates/css/src/lib.rs`
4. `crates/css/modules/values_units/src/chapter_4_numbers.rs`
5. `crates/css/orchestrator/src/style.rs`
6. `crates/css/src/parser.rs`
7. `crates/css/modules/syntax/src/lib.rs`
8. `crates/css/orchestrator/src/style.rs`
9. `crates/page_handler/src/state.rs`
10. `crates/css/modules/selectors/src/spec.md`

<details>
<summary>Execution Logs</summary>

```
Running test: CSS Parsing Implementation
Query: how does CSS parsing work
Project: benchmarks/test_repositories/valor
✓ Found 19 results
Retrieved files:
  1. crates/html/src/lib.rs
  2. crates/css/modules/values_units/src/chapter_5_percentages.rs
  3. crates/css/src/lib.rs
  4. crates/css/modules/values_units/src/chapter_4_numbers.rs
  5. crates/css/orchestrator/src/style.rs
  6. crates/css/src/parser.rs
  7. crates/css/modules/syntax/src/lib.rs
  8. crates/css/orchestrator/src/style.rs
  9. crates/page_handler/src/state.rs
  10. crates/css/modules/selectors/src/spec.md
```
</details>

### HTML Parse Error Recovery

**Query**: "how does the parser recover from malformed HTML tags"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 12.5% |
| Recall@10 | 33.3% |
| MRR | 0.500 |
| NDCG@10 | 0.265 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/html/src/parser/html5ever_engine.rs`
2. `crates/html/src/lib.rs`
3. `crates/css/modules/values_units/src/chapter_3_identifiers.rs`
4. `crates/css/modules/values_units/src/chapter_5_percentages.rs`
5. `crates/css/modules/values_units/src/chapter_4_numbers.rs`
6. `crates/js/src/bindings/dom.rs`
7. `crates/js/src/bindings/document/core.rs`
8. `docs/OPACITY_IMPLEMENTATION.md`

<details>
<summary>Execution Logs</summary>

```
Running test: HTML Parse Error Recovery
Query: how does the parser recover from malformed HTML tags
Project: benchmarks/test_repositories/valor
✓ Found 8 results
Retrieved files:
  1. crates/html/src/parser/html5ever_engine.rs
  2. crates/html/src/lib.rs
  3. crates/css/modules/values_units/src/chapter_3_identifiers.rs
  4. crates/css/modules/values_units/src/chapter_5_percentages.rs
  5. crates/css/modules/values_units/src/chapter_4_numbers.rs
  6. crates/js/src/bindings/dom.rs
  7. crates/js/src/bindings/document/core.rs
  8. docs/OPACITY_IMPLEMENTATION.md
```
</details>

### GPU Text Rendering

**Query**: "debug text rendering artifacts on GPU backend"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/renderer/src/backend.rs`

<details>
<summary>Execution Logs</summary>

```
Running test: GPU Text Rendering
Query: debug text rendering artifacts on GPU backend
Project: benchmarks/test_repositories/valor
✓ Found 1 results
Retrieved files:
  1. crates/renderer/src/backend.rs
```
</details>

### Web Font Loading

**Query**: "add support for @font-face and web font loading"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/renderer/wgpu_backend/src/state/text.rs`
2. `crates/renderer/wgpu_backend/src/state/initialization.rs`
3. `crates/renderer/wgpu_backend/src/offscreen.rs`
4. `crates/css/orchestrator/src/style.rs`
5. `crates/renderer/wgpu_backend/src/state.rs`
6. `crates/css/modules/fonts/src/spec.md`

<details>
<summary>Execution Logs</summary>

```
Running test: Web Font Loading
Query: add support for @font-face and web font loading
Project: benchmarks/test_repositories/valor
✓ Found 6 results
Retrieved files:
  1. crates/renderer/wgpu_backend/src/state/text.rs
  2. crates/renderer/wgpu_backend/src/state/initialization.rs
  3. crates/renderer/wgpu_backend/src/offscreen.rs
  4. crates/css/orchestrator/src/style.rs
  5. crates/renderer/wgpu_backend/src/state.rs
  6. crates/css/modules/fonts/src/spec.md
```
</details>

