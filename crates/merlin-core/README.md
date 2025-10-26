# merlin-core

Foundational types, traits, and error handling for the Merlin AI coding assistant.

## Purpose

This crate provides the core abstractions and types used throughout the Merlin system:
- Error handling and result types
- Core data structures for queries, responses, and context
- Configuration types for routing, validation, and caching
- Task types for workflow orchestration
- Streaming event system
- UI event system
- Prompt loading utilities

## Module Structure

### Core Types (`types.rs`)
- `Query` - User query with optional context
- `Response` - LLM response with metadata
- `Context` - Code context with file references
- `FileContext` - Individual file content with metadata
- `TokenUsage` - Token consumption tracking

### Error Handling (`error.rs`, `routing_error.rs`)
- `Error` - Core error type with variants for all failure modes
- `RoutingError` - Routing-specific errors
- `Result<T>` - Type alias for `Result<T, Error>`

### Configuration (`config.rs`)
- `RoutingConfig` - Overall routing configuration
- `TierConfig` - Model tier settings
- `ValidationConfig` - Validation pipeline settings
- `CacheConfig` - Response caching settings
- `ConversationConfig` - Conversation management settings

### Task System (`task.rs`, `task_list.rs`)
- `Task` - Task definition with validation settings
- `TaskResult` - Execution results with metadata
- `TaskAnalysis` - Complexity analysis results
- `TaskDecision` - Self-determination decisions
- `ExecutionContext` - Context for task execution
- `TaskList` - Multi-step workflow structure
- `TaskStep` - Individual workflow steps

### Streaming System (`streaming/`)
- `StreamingEvent` - Events for streaming responses
- `StreamingChannel` - Channel for streaming events
- `TaskStep` - Streaming task step updates

### UI System (`ui/`)
- `UiEvent` - UI events for terminal display
- `UiChannel` - Channel for UI events
- `TaskProgress` - Task progress tracking

### Traits (`traits.rs`)
- `ModelProvider` - Trait for LLM provider implementations

### Prompts (`prompts/`)
- Utilities for loading and managing system prompts

## Public API

**135 public items** including:
- Error types: `Error`, `RoutingError`, `Result`
- Core types: `Query`, `Response`, `Context`, `FileContext`, `TokenUsage`
- Trait: `ModelProvider`
- Configuration types
- Task types
- Event systems

## Testing Status

**✅ Well-tested**

- **Unit tests**: 6 files with comprehensive coverage
  - `error.rs` - Error conversion and handling
  - `types.rs` - Type serialization and token calculations
  - `task.rs` - Task validation and result handling
  - `task_list.rs` - Workflow orchestration
  - `routing_error.rs` - Error type conversions
  - `streaming/mod.rs` - Event streaming
- **Integration tests**: Via fixtures in `integration-tests` crate
- **Coverage**: Core functionality well-covered

## Code Quality

- ✅ **Documentation**: All public items have comprehensive doc comments
- ✅ **Error handling**: Proper `Result<T, E>` usage throughout
- ✅ **No dead code**: All modules actively used
- ✅ **No clippy violations**: Strict linting compliance
- ✅ **No TODOs**: Implementation complete

## Dependencies

- `serde` - Serialization support
- `thiserror` - Error type derivation
- `tokio` - Async runtime and channels
- `url` - URL parsing for file contexts

## Issues and Recommendations

**None** - This crate is well-maintained with excellent test coverage and documentation.
