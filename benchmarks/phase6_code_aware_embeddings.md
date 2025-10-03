# Context Fetching Benchmark Report

**Date**: 2025-10-03 16:18:33

**Test Cases**: 20

## Summary

| Metric | Value |
|--------|-------|
| Precision@3 | 23.3% |
| Precision@5 | 25.0% |
| Precision@10 | 19.0% |
| Recall@10 | 47.0% |
| MRR | 0.400 |
| NDCG@10 | 0.390 |
| Exclusion Rate | -17.5% |
| Critical in Top-3 | 15.0% |
| High in Top-5 | 25.8% |

## Individual Test Cases

---

# Benchmark: CSS Animation Performance

**Query**: "why are CSS animations dropping frames on transforms"

**Description**: Debug janky CSS animations

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  20.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    25.0%
- **MRR**:          0.250
- **NDCG@10**:      0.192
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\transforms\README.md:1-27 ❌ (not expected) (score: 0.867)
3. benchmarks/test_repositories/valor\crates\css\modules\transforms\src\spec.md:1255-1304 ❌ (not expected) (score: 0.851)
4. benchmarks/test_repositories/valor\crates\css\modules\transforms\src\lib.rs:1-4 ✅ (expected: Critical) (score: 0.818)
5. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_5_percentages.rs:1-22 ❌ (not expected) (score: 0.806)
6. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_4_numbers.rs:1-27 ❌ (not expected) (score: 0.797)
7. benchmarks/test_repositories/valor\crates\css\modules\conditional_rules\src\spec.md:75-104 ❌ (not expected) (score: 0.784)
8. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state.rs:1737-1770 ❌ (not expected) (score: 0.768)
9. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\part_8_3_1_collapsing_margins.rs:34-62 ❌ (not expected) (score: 0.767)
10. benchmarks/test_repositories/valor\crates\css\modules\transforms\src\spec.md:1376-1390 ❌ (not expected) (score: 0.710)

## Missing Expected Files

- **crates/css/modules/core/src/lib.rs** (Critical): Core CSS property handling
- **crates/renderer/src/lib.rs** (High): Rendering pipeline for animations
- **crates/page_handler/src/lib.rs** (Medium): Animation frame coordination

---

# Benchmark: Async Rendering Pipeline

**Query**: "implement async rendering to prevent main thread blocking"

**Description**: Implement async rendering to avoid blocking

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  20.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    25.0%
- **MRR**:          0.200
- **NDCG@10**:      0.172
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\valor\src\state.rs:10-27 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\initialization.rs:36-65 ❌ (not expected) (score: 0.647)
3. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (not expected) (score: 0.501)
4. benchmarks/test_repositories/valor\crates\valor\src\main.rs:230-261 ❌ (not expected) (score: 0.411)
5. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state.rs:1737-1770 ✅ (expected: Critical) (score: 0.401)

## Missing Expected Files

- **crates/renderer/src/renderer.rs** (Critical): Main renderer implementation
- **crates/page_handler/src/lib.rs** (High): Page rendering coordination
- **crates/renderer/src/lib.rs** (Medium): Renderer module exports

---

# Benchmark: Box Model Bug Fix

**Query**: "where is margin collapse calculated for adjacent elements"

**Description**: Bug in margin collapse calculation

## Metrics

- **Precision@3**:  100.0%
- **Precision@5**:  100.0%
- **Precision@10**: 80.0%
- **Recall@10**:    200.0%
- **Recall@20**:    225.0%
- **MRR**:          1.000
- **NDCG@10**:      1.615
- **Exclusion**:    100.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\part_8_3_1_collapsing_margins.rs:34-62 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:465-488 ❌ (not expected) (score: 0.746)
3. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:160-187 ✅ (expected: High) (score: 0.652)
4. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:411-440 ❌ (not expected) (score: 0.642)
5. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:105-122 ❌ (not expected) (score: 0.604)
6. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:1492-1534 ❌ (not expected) (score: 0.596)
7. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:208-231 ❌ (not expected) (score: 0.552)
8. benchmarks/test_repositories/valor\crates\css\modules\core\src\orchestrator\place_child.rs:16-33 ❌ (not expected) (score: 0.544)
9. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\part_8_3_1_collapsing_margins.rs:98-125 ❌ (not expected) (score: 0.518)
10. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:441-464 ❌ (not expected) (score: 0.502)

## Missing Expected Files

- **crates/css/modules/box/src/lib.rs** (Critical): Box module properties
- **crates/css/orchestrator/src/lib.rs** (Medium): CSS orchestration and calculation

---

# Benchmark: Console Logging Implementation

**Query**: "fix console.log output not appearing in debug mode"

**Description**: Bug fix query about console.log not working

## Metrics

- **Precision@3**:  33.3%
- **Precision@5**:  20.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    25.0%
- **MRR**:          0.333
- **NDCG@10**:      0.223
- **Exclusion**:    50.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\orchestrator\src\layout_model.rs:1-14 ❌ (excluded) (score: 0.571)
3. benchmarks/test_repositories/valor\crates\js\src\bindings\logger.rs:1-7 ✅ (expected: Critical) (score: 0.491)
4. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:205-233 ❌ (not expected) (score: 0.454)

## Missing Expected Files

- **crates/js/src/console.rs** (Critical): Console API implementation
- **crates/js/src/runtime.rs** (High): Runtime setup and configuration
- **crates/js/src/lib.rs** (Medium): JS module exports

---

# Benchmark: CSS Parsing Implementation

**Query**: "how does CSS parsing work"

**Description**: Query about how CSS is parsed in the browser engine

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 10.0%
- **Recall@10**:    20.0%
- **Recall@20**:    120.0%
- **MRR**:          0.125
- **NDCG@10**:      0.065
- **Exclusion**:    66.7%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (excluded) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_4_numbers.rs:1-27 ❌ (not expected) (score: 0.913)
3. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_5_percentages.rs:1-22 ❌ (not expected) (score: 0.906)
4. benchmarks/test_repositories/valor\crates\css\modules\display\src\3_display_order\mod.rs:1-29 ❌ (not expected) (score: 0.705)
5. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_3_identifiers.rs:15-52 ❌ (not expected) (score: 0.668)
6. benchmarks/test_repositories/valor\crates\css\modules\style_attr\src\lib.rs:32-56 ❌ (not expected) (score: 0.668)
7. benchmarks/test_repositories/valor\crates\css\modules\core\README.md:124-142 ❌ (not expected) (score: 0.603)
8. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1265-1299 ✅ (expected: Medium) (score: 0.594)
9. benchmarks/test_repositories/valor\crates\css\modules\style_attr\src\spec.md:280-309 ❌ (not expected) (score: 0.584)
10. benchmarks/test_repositories/valor\crates\css\src\parser.rs:55-84 ❌ (not expected) (score: 0.560)

## Missing Expected Files

- **crates/css/modules/cascade/src/lib.rs** (Critical): CSS cascade and parsing logic
- **crates/css/modules/core/src/lib.rs** (Critical): Core CSS module implementation
- **crates/css/orchestrator/src/lib.rs** (High): CSS orchestration and coordination
- **crates/css/modules/box/src/lib.rs** (Medium): CSS box model properties

---

# Benchmark: DOM Mutation Performance

**Query**: "optimize performance of frequent DOM appendChild calls"

**Description**: Optimize DOM mutation operations

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    75.0%
- **MRR**:          0.167
- **NDCG@10**:      0.127
- **Exclusion**:    -350.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\page_handler\src\state.rs:146-171 ❌ (not expected) (score: 0.713)
3. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (excluded) (score: 0.683)
4. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\lib.rs:22-52 ❌ (excluded) (score: 0.665)
5. benchmarks/test_repositories/valor\crates\html\src\parser\mod.rs:175-207 ❌ (not expected) (score: 0.595)
6. benchmarks/test_repositories/valor\crates\js\src\bindings\dom.rs:182-202 ✅ (expected: High) (score: 0.560)
7. benchmarks/test_repositories/valor\crates\css\modules\display\src\2_box_layout_modes\part_2_5_box_generation.rs:16-55 ❌ (not expected) (score: 0.549)
8. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:160-187 ❌ (not expected) (score: 0.548)
9. benchmarks/test_repositories/valor\crates\css\modules\display\src\3_display_order\mod.rs:1-29 ❌ (not expected) (score: 0.545)
10. benchmarks/test_repositories/valor\CLAUDE.md:77-99 ❌ (not expected) (score: 0.544)

## Missing Expected Files

- **crates/page_handler/src/document.rs** (Critical): Document mutation interface
- **crates/page_handler/src/updater.rs** (Medium): DOM update handling

---

# Benchmark: DOM Tree Management

**Query**: "where is the DOM tree built and modified"

**Description**: Query about DOM tree structure and manipulation

## Metrics

- **Precision@3**:  33.3%
- **Precision@5**:  20.0%
- **Precision@10**: 10.0%
- **Recall@10**:    20.0%
- **Recall@20**:    20.0%
- **MRR**:          1.000
- **NDCG@10**:      0.195
- **Exclusion**:    -66.7%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ✅ (expected: Medium) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\page_handler\src\state.rs:146-171 ❌ (not expected) (score: 0.745)
3. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:952-977 ❌ (excluded) (score: 0.742)
4. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:37-72 ❌ (not expected) (score: 0.641)
5. benchmarks/test_repositories/valor\CLAUDE.md:77-99 ❌ (not expected) (score: 0.547)
6. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:409-442 ❌ (not expected) (score: 0.467)
7. benchmarks/test_repositories/valor\crates\css\modules\display\src\3_display_order\mod.rs:1-29 ❌ (not expected) (score: 0.466)
8. benchmarks/test_repositories/valor\crates\css\orchestrator\src\lib.rs:31-82 ❌ (not expected) (score: 0.446)
9. benchmarks/test_repositories/valor\crates\renderer\src\paint\mod.rs:1-12 ❌ (excluded) (score: 0.436)

## Missing Expected Files

- **crates/html/src/parser/html5ever_engine.rs** (Critical): HTML5 parser engine that builds the DOM tree
- **crates/html/src/dom/mod.rs** (Critical): DOM node structure and tree representation
- **crates/html/src/parser/mod.rs** (High): HTML parser module
- **crates/page_handler/src/document.rs** (High): Document interface for DOM manipulation

---

# Benchmark: Event Delegation System

**Query**: "add event delegation support for click handlers"

**Description**: Implementation of event bubbling and capture

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 20.0%
- **Recall@10**:    50.0%
- **Recall@20**:    50.0%
- **MRR**:          0.111
- **NDCG@10**:      0.278
- **Exclusion**:    -200.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\js\src\bindings\logger.rs:1-7 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (excluded) (score: 0.985)
3. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\lib.rs:22-52 ❌ (excluded) (score: 0.964)
4. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (not expected) (score: 0.833)
5. benchmarks/test_repositories/valor\crates\renderer\src\lib.rs:1-22 ❌ (not expected) (score: 0.787)
6. benchmarks/test_repositories/valor\crates\js\js_engine_v8\src\lib.rs:98-126 ❌ (not expected) (score: 0.776)
7. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:338-358 ❌ (not expected) (score: 0.770)
8. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\error.rs:10-31 ❌ (not expected) (score: 0.766)
9. benchmarks/test_repositories/valor\crates\js\src\bindings\dom.rs:182-202 ✅ (expected: Critical) (score: 0.760)
10. benchmarks/test_repositories/valor\crates\js\src\bindings\dom.rs:121-158 ❌ (not expected) (score: 0.755)

## Missing Expected Files

- **crates/page_handler/src/lib.rs** (High): Event handling coordination
- **crates/html/src/dom/mod.rs** (High): DOM tree for event propagation
- **crates/js/src/runtime.rs** (Medium): Runtime event loop

---

# Benchmark: Fetch API Implementation

**Query**: "implement the fetch() API for network requests"

**Description**: Feature implementation query

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  40.0%
- **Precision@10**: 30.0%
- **Recall@10**:    100.0%
- **Recall@20**:    100.0%
- **MRR**:          0.250
- **NDCG@10**:      0.669
- **Exclusion**:    50.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     50.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\js\src\bindings\logger.rs:1-7 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (not expected) (score: 0.936)
3. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:73-126 ❌ (excluded) (score: 0.921)
4. benchmarks/test_repositories/valor\crates\js\src\bindings\net.rs:13-30 ✅ (expected: Critical) (score: 0.921)
5. benchmarks/test_repositories/valor\crates\js\src\bindings\net.rs:88-127 ❌ (not expected) (score: 0.918)
6. benchmarks/test_repositories/valor\crates\js\src\bindings\net.rs:62-82 ❌ (not expected) (score: 0.854)
7. benchmarks/test_repositories/valor\crates\js\src\bindings\document_helpers.rs:75-101 ❌ (not expected) (score: 0.841)
8. benchmarks/test_repositories/valor\crates\js\src\bindings\util.rs:9-32 ❌ (not expected) (score: 0.825)
9. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\initialization.rs:36-65 ❌ (not expected) (score: 0.794)
10. benchmarks/test_repositories/valor\crates\js\src\bindings\document\query.rs:153-190 ❌ (not expected) (score: 0.783)

## Missing Expected Files

- **crates/js/src/runtime.rs** (High): Runtime integration for async operations
- **crates/js/src/lib.rs** (Medium): JS module structure

---

# Benchmark: Web Font Loading

**Query**: "add support for @font-face and web font loading"

**Description**: Implement web font loading and fallbacks

## Metrics

- **Precision@3**:  66.7%
- **Precision@5**:  40.0%
- **Precision@10**: 60.0%
- **Recall@10**:    150.0%
- **Recall@20**:    200.0%
- **MRR**:          1.000
- **NDCG@10**:      1.307
- **Exclusion**:    100.0%
- **Critical in Top-3**: 100.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:7318-7333 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\text.rs:21-36 ✅ (expected: Critical) (score: 0.762)
3. benchmarks/test_repositories/valor\crates\css\modules\core\src\10_visual_details\spec.md:1463-1487 ❌ (not expected) (score: 0.689)
4. benchmarks/test_repositories/valor\crates\css\src\layout_helpers.rs:1-22 ❌ (not expected) (score: 0.592)
5. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:2820-2851 ❌ (not expected) (score: 0.549)
6. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\lib.rs:1-4 ❌ (not expected) (score: 0.529)
7. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:7334-7344 ❌ (not expected) (score: 0.488)
8. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:3058-3091 ❌ (not expected) (score: 0.466)
9. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:2720-2851 ❌ (not expected) (score: 0.461)
10. benchmarks/test_repositories/valor\crates\renderer\src\display_list.rs:43-76 ❌ (not expected) (score: 0.451)

## Missing Expected Files

- **crates/css/modules/text/src/lib.rs** (High): Text properties including font-family
- **crates/css/orchestrator/src/lib.rs** (Medium): CSS property resolution

---

# Benchmark: GPU Text Rendering

**Query**: "debug text rendering artifacts on GPU backend"

**Description**: Bug fix for text rendering artifacts

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 0.0%
- **Recall@10**:    0.0%
- **Recall@20**:    0.0%
- **MRR**:          0.000
- **NDCG@10**:      0.000
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (not expected) (score: 1.000)

## Missing Expected Files

- **crates/renderer/wgpu_backend/src/state/text.rs** (Critical): GPU text rendering implementation
- **crates/renderer/wgpu_backend/src/state.rs** (Critical): GPU state management
- **crates/renderer/wgpu_backend/src/lib.rs** (High): WGPU backend entry point
- **crates/css/modules/text/src/lib.rs** (Medium): Text CSS properties

---

# Benchmark: CSS Grid Layout

**Query**: "implement CSS Grid layout algorithm"

**Description**: Add CSS Grid support

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    50.0%
- **MRR**:          0.111
- **NDCG@10**:      0.114
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\display\src\2_box_layout_modes\mod.rs:1-7 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\page_handler\src\snapshots.rs:1-16 ❌ (not expected) (score: 0.796)
3. benchmarks/test_repositories/valor\crates\css\orchestrator\src\layout_model.rs:1-14 ❌ (not expected) (score: 0.664)
4. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:587-608 ❌ (not expected) (score: 0.516)
5. benchmarks/test_repositories/valor\crates\css\modules\core\README.md:59-77 ❌ (not expected) (score: 0.512)
6. benchmarks/test_repositories/valor\crates\css\modules\position\src\lib.rs:1-4 ❌ (not expected) (score: 0.497)
7. benchmarks/test_repositories/valor\CLAUDE.md:46-61 ❌ (not expected) (score: 0.486)
8. benchmarks/test_repositories/valor\crates\valor\src\layout_compare_core.rs:166-183 ❌ (not expected) (score: 0.480)
9. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:285-314 ✅ (expected: High) (score: 0.472)
10. benchmarks/test_repositories/valor\crates\css\orchestrator\src\data.rs:1-2 ❌ (not expected) (score: 0.457)

## Missing Expected Files

- **crates/css/modules/display/src/lib.rs** (Critical): Display property handling including grid
- **crates/css/orchestrator/src/lib.rs** (Medium): Layout orchestration

---

# Benchmark: HTML Parse Error Recovery

**Query**: "how does the parser recover from malformed HTML tags"

**Description**: Improve error recovery in HTML parser

## Metrics

- **Precision@3**:  100.0%
- **Precision@5**:  60.0%
- **Precision@10**: 30.0%
- **Recall@10**:    75.0%
- **Recall@20**:    75.0%
- **MRR**:          1.000
- **NDCG@10**:      0.892
- **Exclusion**:    50.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:73-126 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ✅ (expected: High) (score: 0.816)
3. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:291-332 ❌ (not expected) (score: 0.556)
4. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_4_numbers.rs:1-27 ❌ (not expected) (score: 0.540)
5. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_5_percentages.rs:1-22 ❌ (not expected) (score: 0.537)
6. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_3_identifiers.rs:15-52 ❌ (not expected) (score: 0.515)
7. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\lib.rs:22-52 ❌ (excluded) (score: 0.473)
8. benchmarks/test_repositories/valor\crates\js\src\bindings\document\core.rs:288-305 ❌ (not expected) (score: 0.440)

## Missing Expected Files

- **crates/html/src/parser/mod.rs** (Critical): Parser module coordination
- **crates/html/src/dom/mod.rs** (Medium): DOM construction from parsed HTML

---

# Benchmark: JavaScript Runtime Integration

**Query**: "how does the JavaScript runtime integrate with the DOM"

**Description**: Query about JavaScript execution and runtime setup

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 0.0%
- **Recall@10**:    0.0%
- **Recall@20**:    20.0%
- **MRR**:          0.056
- **NDCG@10**:      0.000
- **Exclusion**:    -150.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\page_handler\src\runtime.rs:16-41 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:73-126 ❌ (not expected) (score: 0.805)
3. benchmarks/test_repositories/valor\crates\js\js_engine_v8\src\lib.rs:10-45 ❌ (not expected) (score: 0.795)
4. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (not expected) (score: 0.762)
5. benchmarks/test_repositories/valor\crates\valor\src\factory.rs:21-46 ❌ (not expected) (score: 0.719)
6. benchmarks/test_repositories/valor\crates\js\js_engine_v8\src\lib.rs:72-97 ❌ (not expected) (score: 0.668)
7. benchmarks/test_repositories/valor\crates\valor\src\lib.rs:1-7 ❌ (not expected) (score: 0.614)
8. benchmarks/test_repositories/valor\crates\css\modules\display\src\3_display_order\mod.rs:1-29 ❌ (excluded) (score: 0.593)
9. benchmarks/test_repositories/valor\crates\js\src\bindings\document\core.rs:288-305 ❌ (not expected) (score: 0.570)
10. benchmarks/test_repositories/valor\crates\js\src\bindings\document\query.rs:123-150 ❌ (not expected) (score: 0.560)

## Missing Expected Files

- **crates/js/src/runtime.rs** (Critical): JavaScript runtime implementation
- **crates/js/src/lib.rs** (Critical): JS module entry point
- **crates/js/src/dom_index.rs** (High): DOM indexing for JS access
- **crates/page_handler/src/lib.rs** (Medium): Page handler coordination

---

# Benchmark: Layout Engine

**Query**: "implement flexbox layout algorithm"

**Description**: Query about the layout engine and box model

## Metrics

- **Precision@3**:  66.7%
- **Precision@5**:  60.0%
- **Precision@10**: 30.0%
- **Recall@10**:    50.0%
- **Recall@20**:    50.0%
- **MRR**:          1.000
- **NDCG@10**:      0.572
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\src\layout_helpers.rs:1-22 ✅ (expected: Medium) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\display\src\2_box_layout_modes\mod.rs:1-7 ✅ (expected: High) (score: 0.668)
3. benchmarks/test_repositories/valor\crates\css\modules\flexbox\README.md:1-27 ❌ (not expected) (score: 0.623)
4. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\lib.rs:1-32 ✅ (expected: Critical) (score: 0.576)
5. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:293-332 ❌ (not expected) (score: 0.570)
6. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:587-608 ❌ (not expected) (score: 0.530)
7. benchmarks/test_repositories/valor\crates\css\modules\core\src\10_visual_details\part_10_6_3_height_of_blocks.rs:598-617 ❌ (not expected) (score: 0.477)
8. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:950-995 ❌ (not expected) (score: 0.477)
9. benchmarks/test_repositories/valor\crates\css\modules\core\README.md:59-77 ❌ (not expected) (score: 0.406)
10. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:98-128 ❌ (not expected) (score: 0.404)

## Missing Expected Files

- **crates/css/modules/core/src/8_box_model** (Critical): CSS box model implementation
- **crates/css/modules/position/src/lib.rs** (Medium): CSS positioning properties
- **crates/renderer/src/lib.rs** (Low): Renderer module structure

---

# Benchmark: JavaScript Module System

**Query**: "how are ES6 modules loaded and executed"

**Description**: ES modules implementation

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 0.0%
- **Recall@10**:    0.0%
- **Recall@20**:    0.0%
- **MRR**:          0.000
- **NDCG@10**:      0.000
- **Exclusion**:    -500.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\js\js_engine_v8\src\lib.rs:10-45 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\core\README.md:124-142 ❌ (excluded) (score: 0.623)
3. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\lib.rs:1-26 ❌ (not expected) (score: 0.618)
4. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:338-358 ❌ (not expected) (score: 0.604)
5. benchmarks/test_repositories/valor\crates\css\modules\cascade\README.md:50-67 ❌ (not expected) (score: 0.588)
6. benchmarks/test_repositories/valor\crates\css\modules\flexbox\README.md:50-67 ❌ (not expected) (score: 0.587)
7. benchmarks/test_repositories/valor\crates\css\modules\values_units\README.md:50-67 ❌ (not expected) (score: 0.586)
8. benchmarks/test_repositories/valor\crates\css\modules\backgrounds_borders\README.md:50-67 ❌ (not expected) (score: 0.584)
9. benchmarks/test_repositories/valor\crates\css\modules\media_queries\README.md:50-67 ❌ (not expected) (score: 0.583)
10. benchmarks/test_repositories/valor\crates\css\modules\position\README.md:50-67 ❌ (not expected) (score: 0.583)

## Missing Expected Files

- **crates/js/src/modules.rs** (Critical): Module system implementation
- **crates/js/src/runtime.rs** (Critical): Runtime module execution
- **crates/js/src/lib.rs** (High): JS module entry point

---

# Benchmark: Rendering Pipeline

**Query**: "fix the rendering pipeline to handle text layout"

**Description**: Query about the rendering pipeline and WGPU integration

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  0.0%
- **Precision@10**: 0.0%
- **Recall@10**:    0.0%
- **Recall@20**:    0.0%
- **MRR**:          0.000
- **NDCG@10**:      0.000
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\src\layout_helpers.rs:1-22 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\text.rs:21-36 ❌ (not expected) (score: 0.567)
3. benchmarks/test_repositories/valor\crates\css\orchestrator\src\layout.rs:22-67 ❌ (not expected) (score: 0.520)

## Missing Expected Files

- **crates/renderer/wgpu_backend/src/lib.rs** (Critical): Main WGPU rendering backend implementation
- **crates/renderer/wgpu_backend/src/state.rs** (Critical): WGPU state management and rendering pipeline
- **crates/renderer/wgpu_backend/src/text.rs** (High): Text rendering logic
- **crates/renderer/src/renderer.rs** (High): Main renderer implementation
- **crates/renderer/src/lib.rs** (Medium): Renderer module exports

---

# Benchmark: CSS Selector Performance

**Query**: "optimize CSS selector matching for large DOMs"

**Description**: Performance optimization query

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  20.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    125.0%
- **MRR**:          0.200
- **NDCG@10**:      0.172
- **Exclusion**:    50.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:3314-3369 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1081-1125 ❌ (not expected) (score: 0.921)
3. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:952-977 ❌ (not expected) (score: 0.904)
4. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1388-1422 ❌ (not expected) (score: 0.891)
5. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\lib.rs:22-52 ✅ (expected: Critical) (score: 0.833)
6. benchmarks/test_repositories/valor\crates\css\modules\selectors\README.md:1-27 ❌ (not expected) (score: 0.818)
7. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1160-1192 ❌ (not expected) (score: 0.810)
8. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:454-490 ❌ (not expected) (score: 0.790)
9. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1050-1080 ❌ (not expected) (score: 0.768)
10. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:2888-2916 ❌ (not expected) (score: 0.766)

## Missing Expected Files

- **crates/css/modules/selectors/src/matching.rs** (Critical): Selector matching algorithm
- **crates/css/modules/cascade/src/lib.rs** (High): Cascade algorithm with selector matching
- **crates/html/src/dom/mod.rs** (Medium): DOM structure being queried

---

# Benchmark: Viewport Units Implementation

**Query**: "implement vh and vw viewport-relative units"

**Description**: Implement vh/vw viewport units

## Metrics

- **Precision@3**:  0.0%
- **Precision@5**:  20.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    25.0%
- **MRR**:          0.200
- **NDCG@10**:      0.182
- **Exclusion**:    50.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\values_units\README.md:1-27 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\lib.rs:1-26 ❌ (not expected) (score: 0.892)
3. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_6_dimensions.rs:22-37 ❌ (not expected) (score: 0.766)
4. benchmarks/test_repositories/valor\crates\valor\src\lib.rs:1-7 ❌ (not expected) (score: 0.667)
5. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:160-187 ✅ (expected: Critical) (score: 0.666)
6. benchmarks/test_repositories/valor\crates\renderer\src\lib.rs:1-22 ❌ (not expected) (score: 0.649)
7. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\rectangles.rs:138-156 ❌ (not expected) (score: 0.642)
8. benchmarks/test_repositories/valor\crates\page_handler\src\state.rs:1000-1038 ❌ (not expected) (score: 0.642)
9. benchmarks/test_repositories/valor\docs\OPACITY_IMPLEMENTATION.md:450-476 ❌ (not expected) (score: 0.626)
10. benchmarks/test_repositories/valor\crates\valor\src\test_support.rs:360-386 ❌ (not expected) (score: 0.620)

## Missing Expected Files

- **crates/css/modules/box/src/lib.rs** (High): Size calculations with viewport units
- **crates/css/orchestrator/src/lib.rs** (High): CSS value resolution
- **crates/page_handler/src/lib.rs** (Medium): Viewport size management

---

# Benchmark: Z-Index Stacking Context

**Query**: "fix z-index not working with positioned elements"

**Description**: Debug z-index stacking context issues

## Metrics

- **Precision@3**:  66.7%
- **Precision@5**:  80.0%
- **Precision@10**: 40.0%
- **Recall@10**:    100.0%
- **Recall@20**:    175.0%
- **MRR**:          1.000
- **NDCG@10**:      1.032
- **Exclusion**:    -200.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:2248-2281 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\core\src\10_visual_details\spec.md:1131-1157 ❌ (not expected) (score: 0.860)
3. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:2282-2322 ❌ (not expected) (score: 0.845)
4. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:1089-1115 ❌ (not expected) (score: 0.795)
5. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:498-526 ❌ (not expected) (score: 0.728)
6. benchmarks/test_repositories/valor\crates\css\modules\core\src\10_visual_details\spec.md:1103-1130 ❌ (not expected) (score: 0.688)
7. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:78-101 ❌ (not expected) (score: 0.686)
8. benchmarks/test_repositories/valor\crates\js\src\bindings\document\query.rs:247-269 ❌ (excluded) (score: 0.685)
9. benchmarks/test_repositories/valor\crates\renderer\src\paint\stacking.rs:42-70 ❌ (not expected) (score: 0.685)
10. benchmarks/test_repositories/valor\crates\valor\src\state.rs:10-27 ❌ (not expected) (score: 0.683)

## Missing Expected Files

- **crates/css/modules/position/src/lib.rs** (Critical): Positioning and z-index properties
- **crates/renderer/src/lib.rs** (High): Rendering with stacking order
- **crates/css/modules/display/src/lib.rs** (Medium): Display properties affecting stacking

