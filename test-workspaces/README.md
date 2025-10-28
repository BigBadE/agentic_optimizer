# Test Workspaces

This directory contains pre-made workspaces for integration tests.

## Structure

Each subdirectory is a named workspace that can be referenced in test fixtures:

```json
{
  "setup": {
    "workspace": "simple-typescript"
  }
}
```

## Benefits

1. **Speed**: Embeddings are pre-generated once and reused across tests
2. **Consistency**: All tests use the same workspace state
3. **Read-only**: Prevents accidental test pollution

## Adding New Workspaces

1. Create a new directory under `test-workspaces/`
2. Add your test files
3. Reference it in fixtures with `"workspace": "your-workspace-name"`

## Writable Tests

If a test needs to modify files:

```json
{
  "setup": {
    "workspace": "simple-typescript",
    "needs_write": true
  }
}
```

This copies the workspace to a temp directory for the test.
