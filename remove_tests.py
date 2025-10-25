#!/usr/bin/env python
"""
Script to remove low-value tests based on audit recommendations.
Removes tests by commenting them out with a note about why they were removed.
"""

import re
from pathlib import Path
from typing import Optional, Tuple, List

# List of tests to remove with (file_path, test_name, reason)
TESTS_TO_REMOVE = [
    # integration-tests - already done in task_step_comprehensive.rs

    # merlin-agent - conversation tests
    ("crates/merlin-agent/src/agent/conversation.rs", "test_conversation_manager_creation", "Constructor test only"),

    # merlin-agent - executor tests
    ("crates/merlin-agent/src/agent/executor.rs", "test_agent_executor_creation", "Constructor test only"),

    # merlin-cli - state tests (duplicates)
    ("crates/merlin-cli/src/ui/state.rs", "test_add_conversation_entry", "Duplicates agent::conversation"),
    ("crates/merlin-cli/src/ui/state.rs", "test_clear_conversation_history", "Duplicates agent::conversation"),
    ("crates/merlin-cli/src/ui/state.rs", "test_conversation_entry_role_accessors", "Duplicates agent::conversation"),
    ("crates/merlin-cli/src/ui/state.rs", "test_conversation_history_limit", "Duplicates agent::conversation"),

    # merlin-cli - token usage
    ("crates/merlin-cli/src/lib.rs", "test_token_usage_default", "Constructor test"),

    # merlin-cli - utils
    ("crates/merlin-cli/src/utils.rs", "test_display_response_metrics", "Display formatting test"),

    # merlin-core - error tests (reduce from 5 to 2)
    ("crates/merlin-core/src/error.rs", "test_error_display", "Low value trait test"),
    ("crates/merlin-core/src/error.rs", "test_error_from_io", "Low value trait test"),
    ("crates/merlin-core/src/error.rs", "test_error_from_json", "Low value trait test"),

    # merlin-core - task_list (duplicates integration-tests)
    ("crates/merlin-core/src/task_list.rs", "test_next_pending_step", "Duplicate of integration test"),
    ("crates/merlin-core/src/task_list.rs", "test_task_list_status_updates", "Duplicate of integration test"),
    ("crates/merlin-core/src/task_list.rs", "test_task_list_with_failures", "Duplicate of integration test"),
    ("crates/merlin-core/src/task_list.rs", "test_task_step_failure", "Duplicate of integration test"),
    ("crates/merlin-core/src/task_list.rs", "test_task_step_lifecycle", "Duplicate of integration test"),

    # merlin-core - types (constructor tests)
    ("crates/merlin-core/src/types.rs", "test_context_new", "Constructor test"),
    ("crates/merlin-core/src/types.rs", "test_file_context_new", "Constructor test"),
    ("crates/merlin-core/src/types.rs", "test_query_new", "Constructor test"),
    ("crates/merlin-core/src/types.rs", "test_token_usage_default", "Constructor test"),

    # merlin-context - query::types (enum tests)
    ("crates/merlin-context/src/query/types.rs", "test_action_variants", "Low value enum test"),
    ("crates/merlin-context/src/query/types.rs", "test_complexity_ordering", "Low value enum test"),
    ("crates/merlin-context/src/query/types.rs", "test_scope_variants", "Low value enum test"),
    ("crates/merlin-context/src/query/types.rs", "test_serde_query_intent", "Low value serde test"),

    # merlin-languages - provider tests
    ("crates/merlin-languages/src/provider.rs", "test_symbol_kind_debug", "Trait implementation test"),
    ("crates/merlin-languages/src/provider.rs", "test_symbol_kind_equality", "Trait implementation test"),

    # merlin-languages - language enum
    ("crates/merlin-languages/src/lib.rs", "test_language_clone", "Trait implementation test"),
    ("crates/merlin-languages/src/lib.rs", "test_language_debug", "Trait implementation test"),
    ("crates/merlin-languages/src/lib.rs", "test_language_equality", "Trait implementation test"),

    # merlin-tooling - tool tests (reduce from 10 to 3)
    ("crates/merlin-tooling/src/tool.rs", "test_tool_error_display", "Low value trait test"),
    ("crates/merlin-tooling/src/tool.rs", "test_tool_error_from_io", "Low value trait test"),
    ("crates/merlin-tooling/src/tool.rs", "test_tool_input_serialization", "Low value serde test"),
    ("crates/merlin-tooling/src/tool.rs", "test_tool_output_serialization", "Low value serde test"),
    ("crates/merlin-tooling/src/tool.rs", "test_tool_output_success", "Trivial test"),
    ("crates/merlin-tooling/src/tool.rs", "test_tool_output_success_with_data", "Trivial test"),
    ("crates/merlin-tooling/src/tool.rs", "test_tool_trait_implementation", "Trivial test"),
]

def find_test_function(content: str, test_name: str) -> Optional[Tuple[int, int]]:
    """Find the start and end position of a test function."""
    # Pattern to match #[test] followed by fn test_name
    pattern = rf'#\[test\]\s*(?:#\[cfg_attr.*?\])?\s*fn\s+{re.escape(test_name)}\s*\('

    match = re.search(pattern, content, re.MULTILINE | re.DOTALL)
    if not match:
        return None

    start = match.start()

    # Find the matching closing brace
    brace_count = 0
    in_function = False
    i = match.end()

    while i < len(content):
        if content[i] == '{':
            in_function = True
            brace_count += 1
        elif content[i] == '}':
            brace_count -= 1
            if in_function and brace_count == 0:
                return (start, i + 1)
        i += 1

    return None

def remove_tests(file_path: Path, tests: List[Tuple[str, str]]):
    """Remove tests from a file."""
    if not file_path.exists():
        print(f"Warning: {file_path} not found")
        return 0

    content = file_path.read_text(encoding='utf-8')
    removed_count = 0

    for test_name, reason in tests:
        result = find_test_function(content, test_name)
        if result:
            start, end = result
            # Replace with a comment
            comment = f"// REMOVED: {test_name} - {reason}\n"
            content = content[:start] + comment + content[end:]
            removed_count += 1
            print(f"  Removed {test_name}")
        else:
            print(f"  Warning: Could not find {test_name}")

    if removed_count > 0:
        file_path.write_text(content, encoding='utf-8')

    return removed_count

def main():
    """Main entry point."""
    # Group tests by file
    tests_by_file = {}
    for file_path_str, test_name, reason in TESTS_TO_REMOVE:
        if file_path_str not in tests_by_file:
            tests_by_file[file_path_str] = []
        tests_by_file[file_path_str].append((test_name, reason))

    total_removed = 0

    for file_path_str, tests in tests_by_file.items():
        file_path = Path(file_path_str)
        print(f"\nProcessing {file_path}:")
        removed = remove_tests(file_path, tests)
        total_removed += removed

    print(f"\n{'='*60}")
    print(f"Total tests removed: {total_removed}")
    print(f"{'='*60}")

if __name__ == "__main__":
    main()
