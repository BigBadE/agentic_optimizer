# Context Fetching Benchmark Report

**Date**: 2025-10-03 16:01:55

**Test Cases**: 20

## Summary

| Metric | Value |
|--------|-------|
| Precision@3 | 30.0% |
| Precision@5 | 24.0% |
| Precision@10 | 20.0% |
| Recall@10 | 49.4% |
| MRR | 0.440 |
| NDCG@10 | 0.437 |
| Exclusion Rate | -28.3% |
| Critical in Top-3 | 25.0% |
| High in Top-5 | 23.8% |

## Individual Test Cases

---

# Benchmark: CSS Animation Performance

**Query**: "why are CSS animations dropping frames on transforms"

**Description**: Debug janky CSS animations

## Metrics

- **Precision@3**:  33.3%
- **Precision@5**:  20.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    25.0%
- **MRR**:          0.500
- **NDCG@10**:      0.281
- **Exclusion**:    100.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\transforms\README.md:1-27 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\transforms\src\lib.rs:1-4 ✅ (expected: Critical) (score: 0.966)
3. benchmarks/test_repositories/valor\crates\page_handler\src\snapshots.rs:1-16 ❌ (not expected) (score: 0.728)
4. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:350-378 ❌ (not expected) (score: 0.721)
5. benchmarks/test_repositories/valor\crates\css\modules\transforms\src\spec.md:1255-1304 ❌ (not expected) (score: 0.671)
6. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:489-510 ❌ (not expected) (score: 0.638)
7. benchmarks/test_repositories/valor\crates\css\modules\text\src\lib.rs:1-39 ❌ (not expected) (score: 0.632)
8. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:1254-1314 ❌ (not expected) (score: 0.627)
9. benchmarks/test_repositories/valor\crates\css\modules\transforms\src\spec.md:252-355 ❌ (not expected) (score: 0.624)
10. benchmarks/test_repositories/valor\crates\css\modules\images\src\spec.md:2224-2246 ❌ (not expected) (score: 0.599)

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
- **Precision@5**:  0.0%
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    25.0%
- **MRR**:          0.167
- **NDCG@10**:      0.159
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\valor\src\state.rs:10-27 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\initialization.rs:36-65 ❌ (not expected) (score: 0.647)
3. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (not expected) (score: 0.518)
4. benchmarks/test_repositories/valor\crates\valor\src\main.rs:117-144 ❌ (not expected) (score: 0.410)
5. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:1-36 ❌ (not expected) (score: 0.406)
6. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state.rs:1737-1770 ✅ (expected: Critical) (score: 0.403)
7. benchmarks/test_repositories/valor\crates\valor\src\main.rs:230-261 ❌ (not expected) (score: 0.401)

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
- **Precision@5**:  80.0%
- **Precision@10**: 80.0%
- **Recall@10**:    200.0%
- **Recall@20**:    225.0%
- **MRR**:          1.000
- **NDCG@10**:      1.642
- **Exclusion**:    100.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\part_8_3_1_collapsing_margins.rs:34-62 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:465-488 ❌ (not expected) (score: 0.762)
3. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:411-440 ❌ (not expected) (score: 0.642)
4. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:105-122 ✅ (expected: High) (score: 0.589)
5. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:1492-1534 ❌ (not expected) (score: 0.588)
6. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:489-510 ❌ (not expected) (score: 0.540)
7. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:208-231 ❌ (not expected) (score: 0.532)
8. benchmarks/test_repositories/valor\crates\css\modules\core\src\orchestrator\place_child.rs:16-33 ❌ (not expected) (score: 0.531)
9. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\part_8_3_1_collapsing_margins.rs:98-125 ❌ (not expected) (score: 0.525)
10. benchmarks/test_repositories/valor\crates\css\modules\core\src\8_box_model\spec.md:441-464 ❌ (not expected) (score: 0.505)

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
2. benchmarks/test_repositories/valor\crates\css\modules\style_attr\src\spec.md:417-463 ❌ (excluded) (score: 0.759)
3. benchmarks/test_repositories/valor\crates\js\src\bindings\logger.rs:1-7 ✅ (expected: Critical) (score: 0.476)
4. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:205-233 ❌ (not expected) (score: 0.458)
5. benchmarks/test_repositories/valor\crates\js\js_engine_v8\src\lib.rs:10-45 ❌ (not expected) (score: 0.450)

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
- **Precision@10**: 20.0%
- **Recall@10**:    40.0%
- **Recall@20**:    120.0%
- **MRR**:          0.167
- **NDCG@10**:      0.135
- **Exclusion**:    66.7%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (excluded) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_4_numbers.rs:1-27 ❌ (not expected) (score: 0.878)
3. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_5_percentages.rs:1-22 ❌ (not expected) (score: 0.874)
4. benchmarks/test_repositories/valor\crates\css\src\lib.rs:53-108 ❌ (not expected) (score: 0.836)
5. benchmarks/test_repositories/valor\crates\css\modules\style_attr\src\spec.md:280-309 ❌ (not expected) (score: 0.599)
6. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1265-1299 ✅ (expected: Medium) (score: 0.588)
7. benchmarks/test_repositories/valor\crates\css\orchestrator\src\style.rs:889-956 ❌ (not expected) (score: 0.563)
8. benchmarks/test_repositories/valor\crates\css\src\parser.rs:55-84 ❌ (not expected) (score: 0.558)
9. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:76-101 ❌ (not expected) (score: 0.535)
10. benchmarks/test_repositories/valor\crates\css\modules\images\src\spec.md:2416-2429 ❌ (not expected) (score: 0.532)

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
- **Recall@20**:    50.0%
- **MRR**:          0.143
- **NDCG@10**:      0.119
- **Exclusion**:    -200.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\page_handler\src\state.rs:146-171 ❌ (not expected) (score: 0.681)
3. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:422-475 ❌ (not expected) (score: 0.629)
4. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (excluded) (score: 0.620)
5. benchmarks/test_repositories/valor\crates\html\src\parser\mod.rs:175-207 ❌ (not expected) (score: 0.586)
6. benchmarks/test_repositories/valor\crates\css\orchestrator\src\lib.rs:31-82 ❌ (excluded) (score: 0.585)
7. benchmarks/test_repositories/valor\crates\js\src\bindings\dom.rs:182-202 ✅ (expected: High) (score: 0.537)
8. benchmarks/test_repositories/valor\CLAUDE.md:77-99 ❌ (not expected) (score: 0.528)
9. benchmarks/test_repositories/valor\crates\js\src\dom_index.rs:14-31 ❌ (not expected) (score: 0.503)
10. benchmarks/test_repositories/valor\crates\css\modules\display\src\2_box_layout_modes\part_2_5_box_generation.rs:16-55 ❌ (not expected) (score: 0.501)

## Missing Expected Files

- **crates/html/src/dom/mod.rs** (Critical): DOM tree manipulation
- **crates/page_handler/src/document.rs** (Critical): Document mutation interface
- **crates/page_handler/src/updater.rs** (Medium): DOM update handling

---

# Benchmark: DOM Tree Management

**Query**: "where is the DOM tree built and modified"

**Description**: Query about DOM tree structure and manipulation

## Metrics

- **Precision@3**:  33.3%
- **Precision@5**:  40.0%
- **Precision@10**: 20.0%
- **Recall@10**:    40.0%
- **Recall@20**:    40.0%
- **MRR**:          1.000
- **NDCG@10**:      0.345
- **Exclusion**:    33.3%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     25.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ✅ (expected: Medium) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\page_handler\src\state.rs:146-171 ❌ (not expected) (score: 0.733)
3. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:952-977 ❌ (excluded) (score: 0.716)
4. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:37-72 ❌ (not expected) (score: 0.639)
5. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:422-475 ✅ (expected: Critical) (score: 0.639)
6. benchmarks/test_repositories/valor\CLAUDE.md:77-99 ❌ (not expected) (score: 0.534)
7. benchmarks/test_repositories/valor\crates\css\modules\display\src\3_display_order\mod.rs:1-29 ❌ (not expected) (score: 0.463)

## Missing Expected Files

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
- **Precision@10**: 10.0%
- **Recall@10**:    25.0%
- **Recall@20**:    25.0%
- **MRR**:          0.125
- **NDCG@10**:      0.149
- **Exclusion**:    -200.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\js\src\bindings\logger.rs:1-7 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (excluded) (score: 0.994)
3. benchmarks/test_repositories/valor\crates\css\src\lib.rs:171-230 ❌ (excluded) (score: 0.919)
4. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (not expected) (score: 0.885)
5. benchmarks/test_repositories/valor\crates\js\js_engine_v8\src\lib.rs:98-126 ❌ (not expected) (score: 0.804)
6. benchmarks/test_repositories/valor\crates\page_handler\src\snapshots.rs:1-16 ❌ (not expected) (score: 0.799)
7. benchmarks/test_repositories/valor\ARCHITECTURE_IMPROVEMENTS.md:338-358 ❌ (not expected) (score: 0.766)
8. benchmarks/test_repositories/valor\crates\js\src\bindings\dom.rs:182-202 ✅ (expected: Critical) (score: 0.763)
9. benchmarks/test_repositories/valor\crates\valor\src\main.rs:370-410 ❌ (not expected) (score: 0.759)
10. benchmarks/test_repositories/valor\crates\valor\src\main.rs:319-347 ❌ (not expected) (score: 0.741)

## Missing Expected Files

- **crates/page_handler/src/lib.rs** (High): Event handling coordination
- **crates/html/src/dom/mod.rs** (High): DOM tree for event propagation
- **crates/js/src/runtime.rs** (Medium): Runtime event loop

---

# Benchmark: Fetch API Implementation

**Query**: "implement the fetch() API for network requests"

**Description**: Feature implementation query

## Metrics

- **Precision@3**:  66.7%
- **Precision@5**:  40.0%
- **Precision@10**: 30.0%
- **Recall@10**:    100.0%
- **Recall@20**:    100.0%
- **MRR**:          1.000
- **NDCG@10**:      1.058
- **Exclusion**:    -550.0%
- **Critical in Top-3**: 100.0%
- **High in Top-5**:     50.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\js\src\bindings\net.rs:88-127 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\js\src\bindings\logger.rs:1-7 ❌ (not expected) (score: 0.980)
3. benchmarks/test_repositories/valor\crates\js\src\bindings\net.rs:13-30 ❌ (not expected) (score: 0.971)
4. benchmarks/test_repositories/valor\crates\renderer\src\backend.rs:26-70 ❌ (not expected) (score: 0.940)
5. benchmarks/test_repositories/valor\crates\js\src\bindings\document_helpers.rs:75-101 ❌ (not expected) (score: 0.894)
6. benchmarks/test_repositories/valor\crates\js\src\bindings\net.rs:62-82 ❌ (not expected) (score: 0.893)
7. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:73-126 ❌ (excluded) (score: 0.887)
8. benchmarks/test_repositories/valor\crates\js\src\bindings\util.rs:9-32 ❌ (not expected) (score: 0.878)
9. benchmarks/test_repositories/valor\crates\css\modules\backgrounds_borders\README.md:28-49 ❌ (excluded) (score: 0.801)
10. benchmarks/test_repositories/valor\crates\css\modules\color\README.md:28-49 ❌ (not expected) (score: 0.801)

## Missing Expected Files

- **crates/js/src/runtime.rs** (High): Runtime integration for async operations
- **crates/js/src/lib.rs** (Medium): JS module structure

---

# Benchmark: Web Font Loading

**Query**: "add support for @font-face and web font loading"

**Description**: Implement web font loading and fallbacks

## Metrics

- **Precision@3**:  100.0%
- **Precision@5**:  100.0%
- **Precision@10**: 90.0%
- **Recall@10**:    225.0%
- **Recall@20**:    225.0%
- **MRR**:          1.000
- **NDCG@10**:      1.874
- **Exclusion**:    100.0%
- **Critical in Top-3**: 100.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:7318-7333 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:3231-3278 ❌ (not expected) (score: 0.846)
3. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\text.rs:21-36 ✅ (expected: Critical) (score: 0.703)
4. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:6336-6368 ❌ (not expected) (score: 0.655)
5. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:6177-6195 ❌ (not expected) (score: 0.602)
6. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\lib.rs:1-4 ❌ (not expected) (score: 0.508)
7. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:2820-2851 ❌ (not expected) (score: 0.486)
8. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:7334-7344 ❌ (not expected) (score: 0.451)
9. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:3058-3091 ❌ (not expected) (score: 0.425)
10. benchmarks/test_repositories/valor\crates\css\modules\fonts\src\spec.md:2720-2851 ❌ (not expected) (score: 0.405)

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
- **Recall@20**:    25.0%
- **MRR**:          0.100
- **NDCG@10**:      0.109
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\display\src\2_box_layout_modes\mod.rs:1-7 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\core\README.md:59-77 ❌ (not expected) (score: 0.527)
3. benchmarks/test_repositories/valor\crates\css\modules\position\README.md:1-27 ❌ (not expected) (score: 0.508)
4. benchmarks/test_repositories/valor\crates\css\modules\position\src\lib.rs:1-4 ❌ (not expected) (score: 0.502)
5. benchmarks/test_repositories/valor\CLAUDE.md:46-61 ❌ (not expected) (score: 0.490)
6. benchmarks/test_repositories/valor\crates\css\modules\flexbox\README.md:1-27 ❌ (not expected) (score: 0.485)
7. benchmarks/test_repositories/valor\crates\css\orchestrator\src\data.rs:1-2 ❌ (not expected) (score: 0.452)
8. benchmarks/test_repositories/valor\CLAUDE.md:100-120 ❌ (not expected) (score: 0.450)
9. benchmarks/test_repositories/valor\crates\css\modules\display\src\3_display_order\mod.rs:1-29 ❌ (not expected) (score: 0.435)
10. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\lib.rs:1-32 ✅ (expected: High) (score: 0.407)

## Missing Expected Files

- **crates/css/modules/display/src/lib.rs** (Critical): Display property handling including grid
- **crates/css/modules/core/src/lib.rs** (High): Core layout algorithms
- **crates/css/orchestrator/src/lib.rs** (Medium): Layout orchestration

---

# Benchmark: HTML Parse Error Recovery

**Query**: "how does the parser recover from malformed HTML tags"

**Description**: Improve error recovery in HTML parser

## Metrics

- **Precision@3**:  66.7%
- **Precision@5**:  40.0%
- **Precision@10**: 20.0%
- **Recall@10**:    50.0%
- **Recall@20**:    50.0%
- **MRR**:          1.000
- **NDCG@10**:      0.670
- **Exclusion**:    100.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\html\src\parser\html5ever_engine.rs:73-126 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ✅ (expected: High) (score: 0.907)
3. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_5_percentages.rs:1-22 ❌ (not expected) (score: 0.530)
4. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_3_identifiers.rs:15-52 ❌ (not expected) (score: 0.528)
5. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_4_numbers.rs:1-27 ❌ (not expected) (score: 0.523)
6. benchmarks/test_repositories/valor\crates\js\src\bindings\dom.rs:74-111 ❌ (not expected) (score: 0.478)
7. benchmarks/test_repositories/valor\crates\js\src\bindings\document\core.rs:288-305 ❌ (not expected) (score: 0.447)

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
- **MRR**:          0.062
- **NDCG@10**:      0.000
- **Exclusion**:    -200.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\page_handler\src\runtime.rs:16-41 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\html\src\lib.rs:1-6 ❌ (not expected) (score: 0.744)
3. benchmarks/test_repositories/valor\crates\js\js_engine_v8\src\lib.rs:10-45 ❌ (not expected) (score: 0.721)
4. benchmarks/test_repositories/valor\crates\valor\src\factory.rs:21-46 ❌ (not expected) (score: 0.675)
5. benchmarks/test_repositories/valor\crates\css\src\lib.rs:53-108 ❌ (excluded) (score: 0.648)
6. benchmarks/test_repositories/valor\crates\js\src\bindings\document\core.rs:101-154 ❌ (not expected) (score: 0.567)
7. benchmarks/test_repositories/valor\crates\js\src\bindings\document\query.rs:46-120 ❌ (not expected) (score: 0.538)
8. benchmarks/test_repositories/valor\crates\js\src\bindings\document\core.rs:308-353 ❌ (not expected) (score: 0.532)
9. benchmarks/test_repositories/valor\crates\js\src\bindings\document\core.rs:207-263 ❌ (not expected) (score: 0.520)
10. benchmarks/test_repositories/valor\crates\js\src\bindings\document\core.rs:288-305 ❌ (not expected) (score: 0.518)

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
- **Precision@5**:  40.0%
- **Precision@10**: 20.0%
- **Recall@10**:    33.3%
- **Recall@20**:    33.3%
- **MRR**:          1.000
- **NDCG@10**:      0.518
- **Exclusion**:    100.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     66.7%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\display\src\2_box_layout_modes\mod.rs:1-7 ✅ (expected: High) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\flexbox\README.md:1-27 ❌ (not expected) (score: 0.965)
3. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\lib.rs:1-32 ✅ (expected: Critical) (score: 0.859)
4. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:293-332 ❌ (not expected) (score: 0.833)
5. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:587-608 ❌ (not expected) (score: 0.770)
6. benchmarks/test_repositories/valor\crates\css\modules\core\src\10_visual_details\part_10_6_3_height_of_blocks.rs:598-617 ❌ (not expected) (score: 0.708)
7. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:950-995 ❌ (not expected) (score: 0.668)
8. benchmarks/test_repositories/valor\crates\css\modules\core\README.md:59-77 ❌ (not expected) (score: 0.605)
9. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:98-128 ❌ (not expected) (score: 0.583)
10. benchmarks/test_repositories/valor\crates\css\orchestrator\src\style_model.rs:141-213 ❌ (not expected) (score: 0.558)

## Missing Expected Files

- **crates/css/src/layout_helpers.rs** (Medium): Layout helper functions
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
- **Exclusion**:    -566.7%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     0.0%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\writing_modes\README.md:50-67 ❌ (excluded) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\cascade\README.md:50-67 ❌ (not expected) (score: 0.999)
3. benchmarks/test_repositories/valor\crates\css\modules\values_units\README.md:50-67 ❌ (not expected) (score: 0.996)
4. benchmarks/test_repositories/valor\crates\css\modules\flexbox\README.md:50-67 ❌ (not expected) (score: 0.995)
5. benchmarks/test_repositories/valor\crates\css\modules\media_queries\README.md:50-67 ❌ (not expected) (score: 0.995)
6. benchmarks/test_repositories/valor\crates\css\modules\display\README.md:50-67 ❌ (not expected) (score: 0.994)
7. benchmarks/test_repositories/valor\crates\css\modules\text\README.md:50-67 ❌ (not expected) (score: 0.993)
8. benchmarks/test_repositories/valor\crates\css\modules\syntax\README.md:50-67 ❌ (not expected) (score: 0.991)
9. benchmarks/test_repositories/valor\crates\css\modules\images\README.md:50-67 ❌ (not expected) (score: 0.991)
10. benchmarks/test_repositories/valor\crates\css\modules\text_decoration\README.md:50-67 ❌ (not expected) (score: 0.989)

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

1. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:3314-3369 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:952-977 ❌ (not expected) (score: 0.912)
3. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1081-1125 ❌ (not expected) (score: 0.904)
4. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1388-1422 ❌ (not expected) (score: 0.884)
5. benchmarks/test_repositories/valor\crates\css\modules\selectors\README.md:1-27 ❌ (not expected) (score: 0.819)
6. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:454-490 ❌ (not expected) (score: 0.801)
7. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1160-1192 ❌ (not expected) (score: 0.779)
8. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:1050-1080 ❌ (not expected) (score: 0.750)
9. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:2888-2916 ❌ (not expected) (score: 0.748)
10. benchmarks/test_repositories/valor\crates\css\modules\selectors\src\spec.md:491-552 ❌ (not expected) (score: 0.744)

## Missing Expected Files

- **crates/css/modules/selectors/src/lib.rs** (Critical): Selector matching implementation
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
- **Exclusion**:    100.0%
- **Critical in Top-3**: 0.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\values_units\README.md:1-27 ❌ (not expected) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\lib.rs:1-26 ❌ (not expected) (score: 0.850)
3. benchmarks/test_repositories/valor\crates\css\modules\values_units\src\chapter_6_dimensions.rs:22-37 ❌ (not expected) (score: 0.731)
4. benchmarks/test_repositories/valor\crates\valor\src\lib.rs:1-7 ❌ (not expected) (score: 0.677)
5. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:160-187 ✅ (expected: Critical) (score: 0.626)
6. benchmarks/test_repositories/valor\crates\renderer\wgpu_backend\src\state\rectangles.rs:138-156 ❌ (not expected) (score: 0.625)
7. benchmarks/test_repositories/valor\docs\OPACITY_IMPLEMENTATION.md:450-476 ❌ (not expected) (score: 0.618)
8. benchmarks/test_repositories/valor\crates\valor\src\test_support.rs:360-386 ❌ (not expected) (score: 0.607)
9. benchmarks/test_repositories/valor\crates\page_handler\src\state.rs:1000-1038 ❌ (not expected) (score: 0.606)
10. benchmarks/test_repositories/valor\crates\css\modules\values_units\README.md:68-95 ❌ (not expected) (score: 0.528)

## Missing Expected Files

- **crates/css/modules/box/src/lib.rs** (High): Size calculations with viewport units
- **crates/css/orchestrator/src/lib.rs** (High): CSS value resolution
- **crates/page_handler/src/lib.rs** (Medium): Viewport size management

---

# Benchmark: Z-Index Stacking Context

**Query**: "fix z-index not working with positioned elements"

**Description**: Debug z-index stacking context issues

## Metrics

- **Precision@3**:  100.0%
- **Precision@5**:  80.0%
- **Precision@10**: 50.0%
- **Recall@10**:    125.0%
- **Recall@20**:    175.0%
- **MRR**:          1.000
- **NDCG@10**:      1.281
- **Exclusion**:    -100.0%
- **Critical in Top-3**: 50.0%
- **High in Top-5**:     33.3%

## Top 10 Results

1. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:2248-2281 ✅ (expected: Critical) (score: 1.000)
2. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:2282-2322 ❌ (not expected) (score: 0.802)
3. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:1089-1115 ❌ (not expected) (score: 0.796)
4. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:498-526 ❌ (not expected) (score: 0.719)
5. benchmarks/test_repositories/valor\crates\renderer\src\paint\stacking.rs:42-70 ❌ (not expected) (score: 0.679)
6. benchmarks/test_repositories/valor\crates\css\modules\core\src\lib.rs:78-101 ❌ (not expected) (score: 0.678)
7. benchmarks/test_repositories/valor\crates\valor\src\state.rs:10-27 ❌ (not expected) (score: 0.656)
8. benchmarks/test_repositories/valor\crates\css\modules\core\src\9_visual_formatting\spec.md:1716-1737 ❌ (not expected) (score: 0.649)
9. benchmarks/test_repositories/valor\crates\css\modules\core\src\10_visual_details\spec.md:1103-1130 ❌ (not expected) (score: 0.640)
10. benchmarks/test_repositories/valor\crates\css\modules\flexbox\src\8_single_line_layout\mod.rs:677-735 ❌ (not expected) (score: 0.631)

## Missing Expected Files

- **crates/css/modules/position/src/lib.rs** (Critical): Positioning and z-index properties
- **crates/renderer/src/lib.rs** (High): Rendering with stacking order
- **crates/css/modules/display/src/lib.rs** (Medium): Display properties affecting stacking

