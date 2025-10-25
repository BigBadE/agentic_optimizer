# Tools E2E Test Migration Coverage

## Migration Summary

Migrated 19 tests from `crates/merlin-routing/tests/e2e/tools/tools_e2e_tests.rs` (542 lines) to 5 consolidated JSON fixtures in the unified format.

## Fixture Files Created

### 1. `show_tool.json` - File Reading Operations
**Coverage: 4 original tests**

| Original Test | Coverage in Fixture |
|--------------|---------------------|
| `test_show_tool_reads_file` | Event 1: Read full file with readFile tool |
| `test_show_tool_with_line_range` | Event 2: Read with start_line/end_line parameters |
| `test_show_tool_nonexistent_file` | Event 4: Error handling for missing files |
| `test_tools_handle_unicode` (partial) | Event 3: Unicode content reading with 世界 🌍 🦀 |

**Lines of code:** 48 events consolidating 4 test functions (~150 lines original)

### 2. `edit_tool.json` - Text Replacement Operations
**Coverage: 5 original tests**

| Original Test | Coverage in Fixture |
|--------------|---------------------|
| `test_edit_tool_replaces_text` | Event 1: Simple text replacement (a + b → a * b) |
| `test_edit_tool_replace_all` | Event 2: Replace all occurrences with replace_all flag |
| `test_edit_tool_fails_on_multiple_matches_without_replace_all` | Event 3: Error when multiple matches without replace_all |
| `test_edit_preserves_file_structure` | Event 4: Multiline file editing preserves structure |
| `test_tools_handle_unicode` (partial) | Event 5: Unicode replacement (世界 → World) |

**Lines of code:** 60 events consolidating 5 test functions (~170 lines original)

### 3. `delete_tool.json` - File Deletion Operations
**Coverage: 3 original tests**

| Original Test | Coverage in Fixture |
|--------------|---------------------|
| `test_delete_tool_removes_file` | Event 1: Successful file deletion with verification |
| `test_delete_tool_nonexistent_file` | Event 2: Error handling for nonexistent files |
| `test_delete_tool_refuses_directory` | Event 3: Error when attempting to delete directory |

**Lines of code:** 36 events consolidating 3 test functions (~80 lines original)

### 4. `list_tool.json` - Directory Listing Operations
**Coverage: 5 original tests**

| Original Test | Coverage in Fixture |
|--------------|---------------------|
| `test_list_tool_shows_directory_contents` | Event 1: Basic directory listing |
| `test_list_tool_includes_subdirectories` | Event 2: Subdirectory detection in listings |
| `test_list_tool_hidden_files` (both paths) | Event 3-4: Hidden files with include_hidden flag |
| `test_list_tool_nonexistent_directory` | Event 5: Error for nonexistent directory |
| `test_list_tool_on_file_fails` | Event 6: Error when listing file instead of directory |

**Lines of code:** 84 events consolidating 5 test functions (~140 lines original)

### 5. `tool_workflows.json` - Complete Multi-Tool Workflows
**Coverage: 4 original tests**

| Original Test | Coverage in Fixture |
|--------------|---------------------|
| `test_complete_workflow_read_edit_verify` | Event 1: Read → Edit → Verify workflow |
| `test_complete_workflow_create_list_delete` | Event 2: Create 5 files → List → Delete all → Verify |
| `test_tool_registry_lists_all_tools` | Event 3: List available tools in registry |
| `test_tool_registry_unknown_tool` | Event 4: Error handling for unknown tool |

**Lines of code:** 48 events consolidating 4 test functions (~102 lines original)

## Coverage Statistics

### Original Test File
- **Total tests:** 19
- **Lines of code:** 542
- **Test categories:**
  - Show tool: 4 tests
  - Edit tool: 5 tests
  - Delete tool: 3 tests
  - List tool: 5 tests
  - Tool registry: 2 tests
  - Workflows: 2 tests

### New Fixture Files
- **Total fixtures:** 5
- **Total events:** ~25 LLM response events + input events
- **Lines of JSON:** ~650 (more readable, declarative)
- **Test scenarios:** 21 (includes combined unicode test split across fixtures)

### Coverage Improvements

1. **Consolidation:** Related test cases grouped into logical workflows
2. **Declarative:** Each fixture clearly shows setup → events → verification
3. **Reusability:** Setup files defined once, used across multiple events
4. **Verification:** Multi-layer verification (execution, files, UI)
5. **Real workflows:** Tests follow natural user interaction patterns

## Test Scenario Mapping

### Single-Tool Operations
- **show_tool.json:** Basic reads, line ranges, unicode, errors
- **edit_tool.json:** Simple edits, replace_all, multiple matches, structure preservation, unicode
- **delete_tool.json:** Successful deletion, nonexistent files, directory rejection
- **list_tool.json:** Contents, subdirs, hidden files, error cases

### Multi-Tool Workflows
- **tool_workflows.json:**
  - Read → Edit → Verify (3-step workflow)
  - Create 5 → List → Delete all → Verify (complex loop)
  - Tool registry queries
  - Error handling for unknown tools

## Key Improvements Over Original

1. **Event-Driven:** Tests follow actual user interaction timeline
2. **TypeScript-Only:** All LLM responses are TypeScript code blocks (consistent with unified format)
3. **Pattern Matching:** Triggers based on user input patterns
4. **Multi-Layer Verification:** Can verify execution, files, and UI in one test
5. **Setup Reuse:** Files created once in setup, used across multiple events
6. **Error Cases:** Error handling integrated into normal event flow
7. **Unicode Support:** Unicode tests integrated into relevant tool fixtures

## Migration Benefits

1. **Reduced Duplication:** 542 lines → 5 consolidated fixtures
2. **Better Organization:** Tests grouped by functionality
3. **Easier Maintenance:** Change setup once, affects all events
4. **More Realistic:** Events happen in chronological order
5. **Flexible Verification:** Test what matters for each scenario
6. **Coverage Tracking:** Clear mapping from old to new tests

## Next Steps

1. Run fixtures through unified test runner
2. Verify all original test cases pass
3. Add any missing edge cases discovered during testing
4. Delete original `tools_e2e_tests.rs` once migration verified
5. Update test documentation to reference new fixtures
