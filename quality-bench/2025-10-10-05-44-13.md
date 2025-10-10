# Context Quality Benchmark Results

## Summary

**Test Cases**: 40

| Metric | Value | Target |
|--------|-------|--------|
| Precision@3 | 14.2% | 60% |
| Precision@10 | 8.2% | 55% |
| Recall@10 | 19.3% | 70% |
| MRR | 0.285 | 0.700 |
| NDCG@10 | 0.189 | 0.750 |
| Critical in Top-3 | 18.8% | 65% |

## Individual Test Results

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

### Web Font Loading

**Query**: "add support for @font-face and web font loading"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 16.7% |
| Recall@10 | 25.0% |
| MRR | 1.000 |
| NDCG@10 | 0.474 |
| Critical in Top-3 | 50.0% |

**Top 10 Results**:
1. `crates/renderer/wgpu_backend/src/state/text.rs`
2. `crates/renderer/wgpu_backend/src/state/initialization.rs`
3. `crates/renderer/wgpu_backend/src/offscreen.rs`
4. `crates/css/orchestrator/src/style.rs`
5. `crates/renderer/wgpu_backend/src/state.rs`
6. `crates/css/modules/fonts/src/spec.md`

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

### API Endpoints Implementation

**Query**: "how are API endpoints defined and routed"

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
2. `crates/js/src/bindings/logger.rs`
3. `crates/css/src/lib.rs`
4. `crates/js/src/bindings/net.rs`
5. `crates/valor/src/state.rs`
6. `crates/js/src/dom_index.rs`
7. `crates/js/src/bindings/util.rs`
8. `crates/js/src/bindings/mod.rs`
9. `crates/css/modules/core/src/lib.rs`
10. `crates/html/src/lib.rs`

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

### Rendering Pipeline

**Query**: "fix the rendering pipeline to handle text layout"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/css/src/layout_helpers.rs`
2. `crates/renderer/wgpu_backend/src/state/text.rs`

### Block Formatting Context

**Query**: "how does block formatting context work"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 10.0% |
| Recall@10 | 33.3% |
| MRR | 1.000 |
| NDCG@10 | 0.630 |
| Critical in Top-3 | 100.0% |

**Top 10 Results**:
1. `crates/css/modules/core/src/9_visual_formatting/part_9_4_1_block_formatting_context.rs`
2. `crates/css/modules/display/src/inline_context.rs`
3. `crates/css/modules/core/src/10_visual_details/part_10_6_3_height_of_blocks.rs`
4. `crates/css/modules/flexbox/src/4_flex_formatting_context/mod.rs`
5. `crates/css/modules/core/src/10_visual_details/part_10_6_3_height_of_blocks.rs`
6. `crates/css/modules/core/src/orchestrator/place_child.rs`
7. `crates/css/modules/core/src/orchestrator/place_child.rs`
8. `crates/renderer/src/display_list.rs`
9. `crates/css/modules/core/src/orchestrator/place_child.rs`
10. `crates/css/modules/core/src/lib.rs`

### CSS Parsing Implementation

**Query**: "how does CSS parsing work"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
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

### CSS Selector Performance

**Query**: "optimize CSS selector matching for large DOMs"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/css/src/lib.rs`
2. `crates/css/orchestrator/src/style.rs`
3. `crates/css/modules/display/src/3_display_order/mod.rs`
4. `crates/css/modules/selectors/src/spec.md`
5. `crates/css/modules/selectors/src/spec.md`

### Viewport Units Implementation

**Query**: "implement vh and vw viewport-relative units"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 10.0% |
| Recall@10 | 25.0% |
| MRR | 0.333 |
| NDCG@10 | 0.264 |
| Critical in Top-3 | 100.0% |

**Top 10 Results**:
1. `crates/css/modules/values_units/src/lib.rs`
2. `crates/css/modules/values_units/src/chapter_6_dimensions.rs`
3. `crates/css/modules/core/src/lib.rs`
4. `crates/renderer/wgpu_backend/src/state/rectangles.rs`
5. `crates/valor/src/test_support.rs`
6. `crates/page_handler/src/state.rs`
7. `docs/OPACITY_IMPLEMENTATION.md`
8. `crates/css/modules/values_units/README.md`
9. `crates/css/modules/core/src/8_box_model/spec.md`
10. `crates/css/modules/core/src/10_visual_details/spec.md`

### JavaScript Module System

**Query**: "how are ES6 modules loaded and executed"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/js/js_engine_v8/src/lib.rs`
2. `crates/css/modules/box/README.md`
3. `crates/css/modules/core/README.md`
4. `crates/css/orchestrator/README.md`
5. `crates/css/modules/variables/README.md`
6. `crates/css/modules/text_decoration/README.md`
7. `crates/css/modules/writing_modes/README.md`
8. `crates/css/modules/cascade/README.md`
9. `crates/css/modules/values_units/README.md`
10. `crates/css/modules/flexbox/README.md`

### Testing Framework

**Query**: "how are tests structured and run"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/valor/src/lib.rs`
2. `crates/renderer/src/paint/builder.rs`
3. `crates/css/modules/flexbox/src/7_axis_and_order/mod.rs`
4. `crates/js/js_engine_v8/src/lib.rs`
5. `crates/valor/src/layout_compare_core.rs`
6. `crates/css/modules/core/README.md`
7. `crates/css/modules/media_queries/README.md`
8. `crates/css/modules/display/README.md`
9. `crates/css/orchestrator/README.md`
10. `crates/css/modules/text_decoration/README.md`

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

### Console Logging Implementation

**Query**: "fix console.log output not appearing in debug mode"

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
2. `crates/js/js_engine_v8/src/lib.rs`
3. `crates/css/modules/style_attr/src/spec.md`
4. `docs/OPACITY_IMPLEMENTATION.md`
5. `ARCHITECTURE_IMPROVEMENTS.md`
6. `docs/PROMPTS.md`

### Caching Strategy

**Query**: "how does the caching system work"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/css/modules/selectors/src/lib.rs`
2. `crates/renderer/wgpu_backend/src/state/text.rs`
3. `crates/js/src/bindings/storage.rs`
4. `crates/valor/src/test_support.rs`
5. `crates/js/src/bindings/document/storage.rs`
6. `crates/renderer/wgpu_backend/src/state.rs`
7. `crates/valor/src/factory.rs`
8. `crates/js/src/bindings/document/storage.rs`
9. `crates/js/src/bindings/util.rs`
10. `crates/css/modules/core/src/9_visual_formatting/part_9_5_floats.rs`

### CSS Animation Performance

**Query**: "why are CSS animations dropping frames on transforms"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/page_handler/src/snapshots.rs`
2. `crates/css/modules/text/src/lib.rs`
3. `crates/css/modules/transforms/README.md`
4. `docs/NEXT_STEPS.md`
5. `crates/css/modules/core/src/8_box_model/spec.md`
6. `crates/css/modules/transforms/src/spec.md`
7. `crates/css/modules/transforms/src/spec.md`
8. `crates/css/modules/display/src/spec.md`
9. `docs/PROMPTS.md`
10. `crates/css/modules/images/src/spec.md`

### Logging System

**Query**: "how does logging work"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/js/src/bindings/logger.rs`
2. `crates/css/modules/core/src/orchestrator/place_child.rs`
3. `crates/js/src/bindings/storage.rs`
4. `crates/js/src/console.rs`
5. `crates/js/src/bindings/util.rs`
6. `crates/valor/src/test_support.rs`
7. `crates/valor/src/test_support.rs`
8. `crates/js/src/bindings/document/query.rs`
9. `crates/css/modules/core/src/orchestrator/mod.rs`
10. `crates/js/js_engine_v8/src/lib.rs`

### Networking Layer

**Query**: "how are network requests handled"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/js/src/bindings/logger.rs`
2. `crates/js/src/bindings/document_helpers.rs`
3. `crates/renderer/src/backend.rs`
4. `crates/js/src/bindings/util.rs`
5. `crates/js/src/bindings/net.rs`
6. `crates/renderer/wgpu_backend/src/state.rs`
7. `crates/html/src/lib.rs`
8. `crates/renderer/wgpu_backend/src/error.rs`
9. `crates/js/src/bindings/document/query.rs`
10. `crates/js/js_engine_v8/src/lib.rs`

### Database Operations

**Query**: "how are database queries executed"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/js/src/bindings/storage.rs`
2. `crates/js/src/bindings/document/query.rs`
3. `crates/js/src/bindings/net.rs`
4. `crates/page_handler/src/snapshots.rs`
5. `crates/js/src/bindings/document_helpers.rs`
6. `crates/js/src/bindings/util.rs`
7. `crates/css/orchestrator/src/layout_model.rs`
8. `crates/css/orchestrator/src/types.rs`
9. `crates/html/src/lib.rs`
10. `crates/valor/src/test_support.rs`

### Async Rendering Pipeline

**Query**: "implement async rendering to prevent main thread blocking"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 11.1% |
| Recall@10 | 25.0% |
| MRR | 0.200 |
| NDCG@10 | 0.184 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/valor/src/state.rs`
2. `crates/renderer/wgpu_backend/src/state/initialization.rs`
3. `crates/renderer/src/backend.rs`
4. `crates/valor/src/main.rs`
5. `crates/renderer/wgpu_backend/src/state.rs`
6. `docs/OPACITY_IMPLEMENTATION.md`
7. `crates/css/modules/text/src/spec.md`
8. `docs/REFACTORING_SUMMARY.md`
9. `ARCHITECTURE_IMPROVEMENTS.md`

### HTML Parse Error Recovery

**Query**: "how does the parser recover from malformed HTML tags"

| Metric | Value |
|--------|-------|
| Precision@3 | 66.7% |
| Precision@10 | 25.0% |
| Recall@10 | 50.0% |
| MRR | 1.000 |
| NDCG@10 | 0.674 |
| Critical in Top-3 | 50.0% |

**Top 10 Results**:
1. `crates/html/src/parser/html5ever_engine.rs`
2. `crates/html/src/lib.rs`
3. `crates/css/modules/values_units/src/chapter_3_identifiers.rs`
4. `crates/css/modules/values_units/src/chapter_5_percentages.rs`
5. `crates/css/modules/values_units/src/chapter_4_numbers.rs`
6. `crates/js/src/bindings/dom.rs`
7. `crates/js/src/bindings/document/core.rs`
8. `docs/OPACITY_IMPLEMENTATION.md`

### Fetch API Implementation

**Query**: "implement the fetch() API for network requests"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 20.0% |
| Recall@10 | 66.7% |
| MRR | 1.000 |
| NDCG@10 | 0.756 |
| Critical in Top-3 | 100.0% |

**Top 10 Results**:
1. `crates/js/src/bindings/net.rs`
2. `crates/js/src/bindings/logger.rs`
3. `crates/renderer/src/backend.rs`
4. `crates/js/src/bindings/document_helpers.rs`
5. `crates/html/src/parser/html5ever_engine.rs`
6. `crates/js/src/bindings/util.rs`
7. `crates/js/src/bindings/document/query.rs`
8. `crates/renderer/wgpu_backend/src/state/initialization.rs`
9. `crates/js/src/runtime.rs`
10. `crates/html/src/lib.rs`

### Error Handling Implementation

**Query**: "how are errors handled and propagated"

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
2. `crates/css/src/lib.rs`
3. `crates/js/src/console.rs`
4. `crates/renderer/wgpu_backend/src/error.rs`
5. `crates/page_handler/src/state.rs`
6. `crates/html/src/dom/mod.rs`
7. `crates/renderer/wgpu_backend/src/state.rs`
8. `crates/renderer/wgpu_backend/src/state/error_scope.rs`
9. `crates/js/js_engine_v8/src/lib.rs`
10. `crates/valor/src/main.rs`

### CSS Positioning

**Query**: "how is relative positioning calculated"

| Metric | Value |
|--------|-------|
| Precision@3 | 66.7% |
| Precision@10 | 30.0% |
| Recall@10 | 100.0% |
| MRR | 1.000 |
| NDCG@10 | 0.837 |
| Critical in Top-3 | 100.0% |

**Top 10 Results**:
1. `crates/css/modules/core/src/9_visual_formatting/part_9_4_3_relative_positioning.rs`
2. `crates/css/modules/core/src/lib.rs`
3. `crates/css/orchestrator/src/style_model.rs`
4. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
5. `crates/css/modules/cascade/src/lib.rs`
6. `crates/css/modules/core/src/lib.rs`
7. `crates/css/modules/core/src/10_visual_details/part_10_6_3_height_of_blocks.rs`
8. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
9. `crates/css/modules/core/src/9_visual_formatting/spec.md`
10. `crates/css/modules/core/src/9_visual_formatting/spec.md`

### Configuration Management

**Query**: "how is configuration loaded and managed"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/js/src/bindings/storage.rs`
2. `crates/renderer/wgpu_backend/src/state.rs`
3. `crates/js/js_engine_v8/src/lib.rs`
4. `docs/MODULE_SPEC_FORMAT.md`
5. `docs/architecture.md`
6. `crates/css/modules/media_queries/README.md`
7. `crates/css/orchestrator/README.md`
8. `crates/css/modules/conditional_rules/README.md`
9. `crates/css/modules/namespaces/README.md`
10. `crates/css/modules/color/README.md`

### Box Model Bug Fix

**Query**: "where is margin collapse calculated for adjacent elements"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 16.7% |
| Recall@10 | 25.0% |
| MRR | 0.500 |
| NDCG@10 | 0.200 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/css/modules/core/src/8_box_model/part_8_3_1_collapsing_margins.rs`
2. `crates/css/modules/core/src/lib.rs`
3. `crates/css/modules/core/src/orchestrator/place_child.rs`
4. `crates/css/modules/core/src/8_box_model/spec.md`
5. `crates/css/modules/core/src/9_visual_formatting/spec.md`
6. `crates/css/modules/core/src/10_visual_details/spec.md`

### JavaScript Runtime Integration

**Query**: "how does the JavaScript runtime integrate with the DOM"

| Metric | Value |
|--------|-------|
| Precision@3 | 33.3% |
| Precision@10 | 20.0% |
| Recall@10 | 40.0% |
| MRR | 0.500 |
| NDCG@10 | 0.346 |
| Critical in Top-3 | 50.0% |

**Top 10 Results**:
1. `crates/page_handler/src/runtime.rs`
2. `crates/js/src/runtime.rs`
3. `crates/html/src/lib.rs`
4. `crates/js/js_engine_v8/src/lib.rs`
5. `crates/valor/src/factory.rs`
6. `crates/css/src/lib.rs`
7. `crates/js/src/bindings/document/core.rs`
8. `crates/js/src/bindings/document/query.rs`
9. `crates/page_handler/src/state.rs`
10. `crates/js/src/bindings/dom.rs`

### Async Rendering Pipeline

**Query**: "implement async rendering to prevent main thread blocking"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/valor/src/state.rs`
2. `crates/renderer/wgpu_backend/src/state/initialization.rs`
3. `crates/renderer/src/backend.rs`
4. `crates/valor/src/main.rs`
5. `crates/renderer/wgpu_backend/src/state.rs`
6. `docs/OPACITY_IMPLEMENTATION.md`
7. `crates/css/modules/text/src/spec.md`
8. `docs/REFACTORING_SUMMARY.md`
9. `ARCHITECTURE_IMPROVEMENTS.md`

### State Management

**Query**: "how is application state managed"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/valor/src/state.rs`
2. `crates/valor/src/main.rs`
3. `crates/css/src/lib.rs`
4. `crates/renderer/wgpu_backend/src/state/initialization.rs`
5. `crates/renderer/wgpu_backend/src/state.rs`
6. `CLAUDE.md`
7. `ARCHITECTURE_IMPROVEMENTS.md`

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

### DOM Tree Management

**Query**: "where is the DOM tree built and modified"

| Metric | Value |
|--------|-------|
| Precision@3 | 66.7% |
| Precision@10 | 28.6% |
| Recall@10 | 40.0% |
| MRR | 1.000 |
| NDCG@10 | 0.350 |
| Critical in Top-3 | 50.0% |

**Top 10 Results**:
1. `crates/html/src/lib.rs`
2. `crates/page_handler/src/state.rs`
3. `crates/html/src/parser/html5ever_engine.rs`
4. `crates/css/modules/display/src/3_display_order/mod.rs`
5. `crates/css/modules/selectors/src/spec.md`
6. `ARCHITECTURE_IMPROVEMENTS.md`
7. `CLAUDE.md`

### Z-Index Stacking Context

**Query**: "fix z-index not working with positioned elements"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/js/src/dom_index.rs`
2. `crates/renderer/src/paint/stacking.rs`
3. `crates/css/modules/core/src/lib.rs`
4. `crates/valor/src/state.rs`
5. `crates/css/modules/flexbox/src/8_single_line_layout/mod.rs`
6. `crates/js/src/bindings/dom.rs`
7. `crates/js/src/bindings/document/query.rs`
8. `crates/css/modules/selectors/src/lib.rs`
9. `crates/css/modules/core/src/orchestrator/place_child.rs`
10. `crates/valor/src/main.rs`

### Layout Engine

**Query**: "implement flexbox layout algorithm"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 10.0% |
| Recall@10 | 16.7% |
| MRR | 0.250 |
| NDCG@10 | 0.188 |
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

### DOM Mutation Performance

**Query**: "optimize performance of frequent DOM appendChild calls"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 10.0% |
| Recall@10 | 25.0% |
| MRR | 0.143 |
| NDCG@10 | 0.105 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/html/src/lib.rs`
2. `crates/page_handler/src/state.rs`
3. `crates/renderer/src/backend.rs`
4. `crates/html/src/parser/html5ever_engine.rs`
5. `crates/html/src/parser/mod.rs`
6. `crates/css/orchestrator/src/lib.rs`
7. `crates/js/src/bindings/dom.rs`
8. `crates/js/src/dom_index.rs`
9. `crates/css/modules/display/src/2_box_layout_modes/part_2_5_box_generation.rs`
10. `crates/css/modules/display/src/3_display_order/mod.rs`

### Event Delegation System

**Query**: "add event delegation support for click handlers"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 10.0% |
| Recall@10 | 25.0% |
| MRR | 0.143 |
| NDCG@10 | 0.176 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/js/src/bindings/logger.rs`
2. `crates/renderer/src/backend.rs`
3. `crates/css/src/lib.rs`
4. `crates/html/src/lib.rs`
5. `crates/js/js_engine_v8/src/lib.rs`
6. `crates/page_handler/src/snapshots.rs`
7. `crates/js/src/bindings/dom.rs`
8. `crates/valor/src/main.rs`
9. `crates/renderer/wgpu_backend/src/error.rs`
10. `crates/js/src/bindings/document/query.rs`

### CSS Grid Layout

**Query**: "implement CSS Grid layout algorithm"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 20.0% |
| Recall@10 | 50.0% |
| MRR | 0.250 |
| NDCG@10 | 0.276 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/css/modules/display/src/2_box_layout_modes/mod.rs`
2. `crates/css/modules/display/src/3_display_order/mod.rs`
3. `crates/css/orchestrator/src/style.rs`
4. `crates/css/modules/core/src/lib.rs`
5. `crates/css/modules/core/src/8_box_model/part_8_3_1_collapsing_margins.rs`
6. `crates/css/modules/flexbox/src/lib.rs`
7. `docs/OPACITY_IMPLEMENTATION.md`
8. `docs/OPACITY_IMPLEMENTATION.md`
9. `crates/css/modules/core/README.md`
10. `crates/css/modules/display/src/spec.md`

### Authentication System Implementation

**Query**: "how does user authentication work"

| Metric | Value |
|--------|-------|
| Precision@3 | 0.0% |
| Precision@10 | 0.0% |
| Recall@10 | 0.0% |
| MRR | 0.000 |
| NDCG@10 | 0.000 |
| Critical in Top-3 | 0.0% |

**Top 10 Results**:
1. `crates/css/orchestrator/src/types.rs`
2. `crates/js/src/bindings/logger.rs`
3. `crates/css/modules/selectors/src/lib.rs`
4. `crates/js/src/bindings/net.rs`
5. `crates/js/src/bindings/storage.rs`
6. `crates/js/src/bindings/dom.rs`
7. `crates/css/modules/selectors/src/lib.rs`
8. `crates/css/modules/values_units/src/chapter_3_identifiers.rs`
9. `crates/css/modules/style_attr/src/lib.rs`
10. `crates/js/src/bindings/document/query.rs`

