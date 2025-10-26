# merlin-cli

Command-line interface with Terminal UI for Merlin.

## Purpose

This crate provides the user-facing CLI and interactive Terminal UI (TUI) for the Merlin AI coding assistant. It handles user input, displays real-time progress, and manages interactive sessions.

## Module Structure

### Core
- `main.rs` - Entry point and CLI initialization
- `cli.rs` - Command-line argument parsing
- `handlers.rs` - Command handlers
- `interactive.rs` - Interactive session management
- `config/mod.rs` - Configuration management
- `utils.rs` - Utility functions

### Terminal UI (`ui/`)
- `mod.rs` - Main TUI module
- `event_handler.rs` - Event handling
- `event_source.rs` - `InputEventSource` trait for fixture-based testing
- `input.rs` - User input handling
- `layout.rs` - UI layout
- `persistence.rs` - State persistence
- `scroll.rs` - Scrolling logic
- `state.rs` - UI state management
- `task_manager.rs` - Task management UI
- `theme.rs` - UI theming

### Application Logic (`ui/app/`)
- `tui_app.rs` - Main TUI application
- `event_loop.rs` - Event loop
- `input_handler.rs` - Input processing
- `key_handling.rs` - Keyboard shortcuts
- `lifecycle.rs` - Application lifecycle
- `navigation.rs` - UI navigation
- `task_operations.rs` - Task operations
- `conversation.rs` - Conversation UI
- `test_helpers.rs` - Testing utilities

### Rendering (`ui/renderer/`)
- `helpers.rs` - Rendering helpers
- `task_rendering.rs` - Task display rendering
- `task_tree_builder.rs` - Task tree construction

## Public API

**Note**: This is a binary crate with minimal public API. UI components are used internally.

## Features

### Interactive Terminal UI
- Real-time progress monitoring
- Task tree display
- Scrollable output
- Modal input editing
- Keyboard navigation

### Command-Line Interface
- Multiple commands (run, analyze, validate, etc.)
- Configuration management
- Interactive and non-interactive modes

### TUI Features
- Task tree with hierarchical display
- Focus switching between panels
- Real-time updates
- Comprehensive UI verification via fixtures

## Testing Status

**✅ Well-tested via fixtures**

- **Unit tests**: 1 file with basic tests (main.rs)
- **TUI fixture coverage**: 4 fixtures
  - `tui/basic_navigation.json` - Basic navigation
  - `tui/comprehensive_ui_verification.json` - Full UI verification
  - `tui/task_tree_display.json` - Task tree rendering
  - `tui/focus_switching.json` - Panel focus management
- **Testing approach**: Fixture-based event injection via `InputEventSource` trait
- **Proper separation**: UI tested via fixtures, not direct manipulation

## Code Quality

- ✅ **Documentation**: Public items documented
- ✅ **Error handling**: Proper `Result<T, E>` usage
- ⚠️ **TODOs**: 4 TODOs for planned features:
  - `conversation.rs:83` - Conversation threading
  - `test_helpers.rs:118, 131, 142` - Task hierarchy features
- ✅ **No dead code**: All modules actively used

## Dependencies

- `merlin-core` - Core types
- `merlin-agent` - Agent execution
- `merlin-routing` - Routing logic
- `ratatui` - Terminal UI framework
- `crossterm` - Terminal control
- `clap` - CLI argument parsing
- `tokio` - Async runtime

## Usage

### Interactive Mode
```bash
merlin
```

### Run Command
```bash
merlin run "Add error handling to parser"
```

### Analyze Task
```bash
merlin analyze "Refactor authentication module"
```

### Configuration
```bash
merlin config set groq.enabled true
```

## Issues and Recommendations

### Future Enhancements
1. Implement conversation threading (conversation.rs:83)
2. Implement task hierarchy features (test_helpers.rs)
3. Document these as backlog items

**Otherwise excellent** - Well-tested with comprehensive TUI fixture coverage using proper fixture-based event injection pattern.
