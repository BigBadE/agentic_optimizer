# Tree-Based Output System Design

## Overview

Transform the task output area from a flat text display into a hierarchical tree structure that shows:
- Task execution steps
- Tool calls and their results
- Thinking processes
- Nested subtasks
- Collapsible sections

## Current System

**Problems**:
- Flat text output in TextArea
- No structure or hierarchy
- Hard to navigate long outputs
- Can't collapse/expand sections
- No visual grouping

## Proposed System

### Visual Structure

```
[>] Task: Analyze codebase
  [*] Thinking: Breaking down the task...
  [T] Tool: read_file
    └─ [+] Result: File content loaded (234 lines)
  [*] Thinking: Processing file structure...
  [T] Tool: list_files
    ├─ [+] Found: src/main.rs
    ├─ [+] Found: src/lib.rs
    └─ [+] Found: Cargo.toml
  [>] Output: Analysis complete
    └─ Summary: 3 files analyzed
```

### Data Structure

```rust
#[derive(Clone, Debug)]
enum OutputNode {
    Step {
        id: String,
        step_type: StepType,
        content: String,
        timestamp: Instant,
        children: Vec<OutputNode>,
        collapsed: bool,
    },
    ToolCall {
        id: String,
        tool_name: String,
        args: Value,
        timestamp: Instant,
        result: Option<ToolResult>,
        collapsed: bool,
    },
    Text {
        content: String,
        level: usize,
    },
}

#[derive(Clone, Debug)]
struct ToolResult {
    success: bool,
    content: String,
    timestamp: Instant,
}

#[derive(Clone, Debug)]
enum StepType {
    Thinking,
    ToolCall,
    Output,
    Subtask,
}

struct OutputTree {
    root: Vec<OutputNode>,
    selected_index: usize,
    collapsed_nodes: HashSet<String>,
}
```

### Keyboard Navigation

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up one visible node |
| `↓` / `j` | Move down one visible node |
| `→` / `l` | Expand selected node |
| `←` / `h` | Collapse selected node |
| `Space` | Toggle collapse/expand |
| `Home` | Jump to first node |
| `End` | Jump to last node |
| `PageUp` | Scroll up by page |
| `PageDown` | Scroll down by page |

### Rendering

```rust
fn render_tree(&self, area: Rect, buf: &mut Buffer) {
    let visible_nodes = self.flatten_visible_nodes();
    
    for (idx, (node, depth)) in visible_nodes.iter().enumerate() {
        let y = area.y + idx as u16;
        if y >= area.y + area.height {
            break;
        }
        
        let is_selected = idx == self.selected_index;
        let style = if is_selected {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };
        
        let prefix = self.build_tree_prefix(depth, node);
        let icon = self.get_node_icon(node);
        let content = self.get_node_content(node);
        
        let line = format!("{}{} {}", prefix, icon, content);
        buf.set_string(area.x, y, &line, style);
    }
}
```

### Integration Points

#### 1. Replace TextArea with Custom Widget

**Before**:
```rust
struct TaskDisplay {
    output_area: TextArea<'static>,
    // ...
}
```

**After**:
```rust
struct TaskDisplay {
    output_tree: OutputTree,
    // ...
}
```

#### 2. Update Event Handlers

**TaskStepStarted**:
```rust
UiEvent::TaskStepStarted { task_id, step_id, step_type, content } => {
    if let Some(task) = self.state.tasks.get_mut(&task_id) {
        task.output_tree.add_step(step_id, step_type, content);
    }
}
```

**ToolCallStarted**:
```rust
UiEvent::ToolCallStarted { task_id, tool, args } => {
    if let Some(task) = self.state.tasks.get_mut(&task_id) {
        task.output_tree.add_tool_call(tool, args);
    }
}
```

**ToolCallCompleted**:
```rust
UiEvent::ToolCallCompleted { task_id, tool, result } => {
    if let Some(task) = self.state.tasks.get_mut(&task_id) {
        task.output_tree.complete_tool_call(tool, result);
    }
}
```

#### 3. Handle Keyboard Input

```rust
if self.focused_pane == FocusedPane::Output {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => {
            if let Some(task_id) = self.state.active_task_id {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.move_up();
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if let Some(task_id) = self.state.active_task_id {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.move_down();
                }
            }
        }
        KeyCode::Right | KeyCode::Char('l') => {
            if let Some(task_id) = self.state.active_task_id {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.expand_selected();
                }
            }
        }
        KeyCode::Left | KeyCode::Char('h') => {
            if let Some(task_id) = self.state.active_task_id {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.collapse_selected();
                }
            }
        }
        KeyCode::Char(' ') => {
            if let Some(task_id) = self.state.active_task_id {
                if let Some(task) = self.state.tasks.get_mut(&task_id) {
                    task.output_tree.toggle_selected();
                }
            }
        }
        _ => {}
    }
}
```

### Features

#### 1. Collapsible Sections
- Click or press Space to collapse/expand
- Collapsed nodes show `[+]` indicator
- Expanded nodes show `[-]` indicator
- Children hidden when parent collapsed

#### 2. Visual Hierarchy
- Indentation shows depth
- Tree lines show parent-child relationships
- Icons indicate node type
- Color coding for different node types

#### 3. Smart Navigation
- Only visible nodes are selectable
- Collapsing parent skips to next visible node
- Expanding shows children immediately
- Scroll follows selection

#### 4. Context Preservation
- Collapsed state persists across renders
- Selection preserved when possible
- Scroll position maintained

### Node Icons

```rust
fn get_node_icon(node: &OutputNode) -> &str {
    match node {
        OutputNode::Step { step_type, collapsed, children, .. } => {
            if !children.is_empty() {
                if *collapsed { "[+]" } else { "[-]" }
            } else {
                match step_type {
                    StepType::Thinking => "[*]",
                    StepType::ToolCall => "[T]",
                    StepType::Output => "[>]",
                    StepType::Subtask => "[S]",
                }
            }
        }
        OutputNode::ToolCall { result, .. } => {
            match result {
                Some(r) if r.success => "[+]",
                Some(_) => "[X]",
                None => "[T]",
            }
        }
        OutputNode::Text { .. } => "  ",
    }
}
```

### Tree Prefix Building

```rust
fn build_tree_prefix(depth: usize, is_last: bool, parent_states: &[bool]) -> String {
    let mut prefix = String::new();
    
    for i in 0..depth {
        if i < parent_states.len() && parent_states[i] {
            prefix.push_str("│ ");
        } else {
            prefix.push_str("  ");
        }
    }
    
    if depth > 0 {
        if is_last {
            prefix.push_str("└─ ");
        } else {
            prefix.push_str("├─ ");
        }
    }
    
    prefix
}
```

### Flattening Algorithm

```rust
fn flatten_visible_nodes(&self) -> Vec<(&OutputNode, usize)> {
    let mut result = Vec::new();
    
    for node in &self.root {
        self.flatten_node(node, 0, &mut result);
    }
    
    result
}

fn flatten_node(&self, node: &OutputNode, depth: usize, result: &mut Vec<(&OutputNode, usize)>) {
    result.push((node, depth));
    
    if !self.is_collapsed(node) {
        if let Some(children) = self.get_children(node) {
            for child in children {
                self.flatten_node(child, depth + 1, result);
            }
        }
    }
}
```

## Implementation Plan

### Phase 1: Core Data Structure
1. Create `OutputNode` enum
2. Create `OutputTree` struct
3. Implement basic tree operations (add, remove, find)
4. Implement flattening algorithm

### Phase 2: Rendering
1. Create custom widget for tree rendering
2. Implement tree prefix building
3. Add icon rendering
4. Add selection highlighting
5. Add scroll support

### Phase 3: Navigation
1. Implement up/down movement
2. Implement expand/collapse
3. Implement Home/End/PageUp/PageDown
4. Handle edge cases (empty tree, single node, etc.)

### Phase 4: Integration
1. Replace TextArea with OutputTree in TaskDisplay
2. Update all event handlers
3. Add keyboard input handling
4. Test with real task execution

### Phase 5: Polish
1. Add color coding
2. Add collapse indicators
3. Optimize rendering for large trees
4. Add auto-scroll to new nodes
5. Persist collapsed state

## Benefits

### ✅ Better Organization
- Clear hierarchy of execution steps
- Visual grouping of related operations
- Easy to see what happened when

### ✅ Improved Navigation
- Keyboard-driven navigation
- Collapse long sections
- Jump to specific steps
- Scroll through large outputs

### ✅ Better UX
- Less overwhelming for complex tasks
- Focus on relevant information
- Expand details when needed
- Clean, structured view

### ✅ Consistency
- Same navigation as task list
- Familiar tree structure
- Consistent keyboard shortcuts
- Unified UI paradigm

## Future Enhancements

### Search/Filter
- Search within output tree
- Filter by node type
- Highlight matches
- Jump to next/previous match

### Copy/Export
- Copy selected node content
- Export tree to text
- Export to JSON
- Copy subtree

### Timestamps
- Show relative times
- Show duration for tool calls
- Highlight slow operations
- Time-based filtering

### Context Menu
- Right-click for options
- Copy node content
- Expand/collapse all children
- Jump to related task

## Migration Strategy

### Backward Compatibility
- Keep TextArea as fallback
- Gradual migration of event handlers
- Support both systems temporarily
- Feature flag for tree mode

### Testing
- Unit tests for tree operations
- Integration tests for rendering
- Manual testing with real tasks
- Performance testing with large trees

## Summary

The tree-based output system provides:
- **Hierarchical structure** for complex task outputs
- **Keyboard navigation** matching the task list
- **Collapsible sections** for managing information
- **Visual clarity** through indentation and icons
- **Better UX** for understanding task execution

This transforms the output from a flat text dump into an organized, navigable structure that makes it easy to understand what the agent is doing and what results it produced.
