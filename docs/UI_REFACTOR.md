# UI Refactoring Plan

## Current Problems

### **Critical Bug: Hierarchical Structure Corruption on Reload**

**Root Cause**: 
```rust
// On load (load_tasks_async)
for (task_id, task_display) in sorted_tasks {
    self.state.tasks.insert(task_id, task_display);
    self.state.task_order.push(task_id);  // ❌ Just appends in chronological order!
}
```

When tasks are loaded from disk:
1. Tasks sorted by `start_time` (chronological)
2. Appended to `task_order` in that order
3. **Ignores parent-child relationships completely!**

Example of corruption:
```
Saved structure:
- Task 1 (10:01)
  - Child A (10:02)
  - Child B (10:05)
- Task 2 (10:03)
- Task 3 (10:04)

Loaded structure (sorted by time):
- Task 1 (10:01)
- Child A (10:02)  
- Task 2 (10:03)   ❌ Should not be between Task 1's children
- Task 3 (10:04)   ❌
- Child B (10:05)  ❌ Separated from parent
```

### **Architectural Issues**

1. **Massive Monolithic File**: `ui/mod.rs` is ~1800 lines handling:
   - TUI initialization
   - State management
   - Event handling
   - Rendering
   - Task ordering logic
   - Input handling
   - File I/O
   - Wrapping logic

2. **Dual State Management**: 
   - `task_order` (Vec) for ordering
   - `tasks` (HashMap) for data
   - These can get out of sync

3. **Complex Ordering Logic**: 
   - Insertion logic scattered across multiple places
   - Different behavior for new tasks vs loaded tasks
   - No single source of truth

4. **No Separation of Concerns**: Everything in one giant struct

## Proposed Solution

### **Phase 1: Fix Critical Bug (Immediate)**

**Fix the load logic to rebuild hierarchical order**:

```rust
pub async fn load_tasks_async(&mut self) {
    if let Ok(Ok(tasks)) = loaded_tasks {
        // 1. Insert all tasks into HashMap
        for (task_id, task_display) in tasks {
            self.state.tasks.insert(task_id, task_display);
        }
        
        // 2. Build hierarchical order from parent relationships
        self.rebuild_task_order();
        
        // 3. Wrap outputs
        for task_id in self.state.task_order.clone() {
            self.auto_wrap_output(task_id);
        }
    }
}

fn rebuild_task_order(&mut self) {
    self.state.task_order.clear();
    
    // Get all tasks sorted by start_time
    let mut all_tasks: Vec<(TaskId, &TaskDisplay)> = 
        self.state.tasks.iter().map(|(&id, task)| (id, task)).collect();
    all_tasks.sort_by_key(|(_, task)| task.start_time);
    
    // Add root tasks first, then recursively add their children
    for (task_id, task) in all_tasks.iter() {
        if task.parent_id.is_none() && !self.state.task_order.contains(task_id) {
            self.add_task_and_descendants(*task_id);
        }
    }
    
    // Handle orphaned tasks (parent doesn't exist)
    for (task_id, _) in all_tasks {
        if !self.state.task_order.contains(&task_id) {
            self.state.task_order.push(task_id);
        }
    }
}

fn add_task_and_descendants(&mut self, task_id: TaskId) {
    self.state.task_order.push(task_id);
    
    // Find all direct children, sorted by start_time
    let mut children: Vec<(TaskId, Instant)> = self.state.tasks
        .iter()
        .filter(|(_, task)| task.parent_id == Some(task_id))
        .map(|(&id, task)| (id, task.start_time))
        .collect();
    children.sort_by_key(|(_, time)| *time);
    
    // Recursively add each child and its descendants
    for (child_id, _) in children {
        self.add_task_and_descendants(child_id);
    }
}
```

### **Phase 2: Refactor into Modules (Medium-term)**

**Proposed Structure**:
```
crates/merlin-routing/src/ui/
├── mod.rs                  (~200 lines)  - Public API, initialization
├── app.rs                  (~300 lines)  - TuiApp struct, main loop
├── state.rs                (~150 lines)  - UiState, TaskDisplay structs
├── events.rs               (existing)    - Event types
├── task_manager.rs         (~300 lines)  - Task ordering, hierarchy logic
├── event_handler.rs        (~400 lines)  - Event handling logic
├── renderer.rs             (~400 lines)  - Rendering logic
├── input.rs                (~200 lines)  - Input handling, wrapping
├── persistence.rs          (~200 lines)  - Save/load logic
└── output_tree.rs          (existing)    - Output tree structure
```

**Module Responsibilities**:

#### **task_manager.rs** (NEW - Most Important)
```rust
pub struct TaskManager {
    tasks: HashMap<TaskId, TaskDisplay>,
    task_order: Vec<TaskId>,  // Hierarchical order
    collapsed_tasks: HashSet<TaskId>,
}

impl TaskManager {
    pub fn new() -> Self { ... }
    
    // Core operations
    pub fn add_task(&mut self, task_id: TaskId, task: TaskDisplay) { ... }
    pub fn remove_task(&mut self, task_id: TaskId) -> Vec<TaskId> { ... }
    pub fn get_task(&self, task_id: TaskId) -> Option<&TaskDisplay> { ... }
    pub fn get_task_mut(&mut self, task_id: TaskId) -> Option<&mut TaskDisplay> { ... }
    
    // Hierarchy operations
    pub fn rebuild_order(&mut self) { ... }
    pub fn get_visible_tasks(&self) -> Vec<TaskId> { ... }
    pub fn get_children(&self, task_id: TaskId) -> Vec<TaskId> { ... }
    pub fn is_descendant_of(&self, task_id: TaskId, ancestor: TaskId) -> bool { ... }
    
    // Collapse operations
    pub fn collapse_task(&mut self, task_id: TaskId) { ... }
    pub fn expand_task(&mut self, task_id: TaskId) { ... }
    pub fn toggle_collapse(&mut self, task_id: TaskId) { ... }
    
    // Iteration
    pub fn iter_tasks(&self) -> impl Iterator<Item = (TaskId, &TaskDisplay)> { ... }
    pub fn task_order(&self) -> &[TaskId] { ... }
}
```

#### **state.rs** (Simplified)
```rust
pub struct UiState {
    pub selected_task_index: usize,
    pub active_task_id: Option<TaskId>,
    pub active_running_tasks: HashSet<TaskId>,
    pub pending_delete_task_id: Option<TaskId>,
    pub loading_tasks: bool,
    pub conversation_history: Vec<ConversationEntry>,
}
```

#### **app.rs** (Main coordinator)
```rust
pub struct TuiApp {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    state: UiState,
    task_manager: TaskManager,  // Delegate to this!
    theme: Theme,
    focused_pane: FocusedPane,
    input_area: TextArea,
    tasks_dir: Option<PathBuf>,
    // ... other fields
}

impl TuiApp {
    pub async fn tick(&mut self) -> Result<bool> {
        self.render()?;
        
        if crossterm::event::poll(Duration::from_millis(100))? {
            let event = crossterm::event::read()?;
            return self.handle_input(event).await;
        }
        
        Ok(false)
    }
    
    // Delegate to task_manager
    pub fn add_task(&mut self, task_id: TaskId, task: TaskDisplay) {
        self.task_manager.add_task(task_id, task);
    }
    
    pub fn delete_selected_task(&mut self) {
        if let Some(&task_id) = self.get_selected_task_id() {
            self.task_manager.remove_task(task_id);
        }
    }
}
```

#### **event_handler.rs**
```rust
impl TuiApp {
    pub fn handle_ui_event(&mut self, event: UiEvent) { ... }
    
    fn handle_task_started(&mut self, task_id: TaskId, description: String, parent_id: Option<TaskId>) { ... }
    fn handle_task_completed(&mut self, task_id: TaskId, result: TaskResult) { ... }
    fn handle_task_output(&mut self, task_id: TaskId, output: String) { ... }
    // ... etc
}
```

#### **renderer.rs**
```rust
impl TuiApp {
    pub fn render(&mut self) -> Result<()> {
        self.terminal.draw(|f| {
            let chunks = Layout::default()...;
            
            self.render_input_area(f, chunks[0]);
            self.render_task_list(f, chunks[1]);
            self.render_output_area(f, chunks[2]);
            self.render_status_bar(f, chunks[3]);
        })?;
        Ok(())
    }
    
    fn render_task_list(&self, f: &mut Frame, area: Rect) { ... }
    fn render_output_area(&self, f: &mut Frame, area: Rect) { ... }
    // ... etc
}
```

#### **persistence.rs**
```rust
pub struct TaskPersistence {
    tasks_dir: PathBuf,
}

impl TaskPersistence {
    pub fn new(tasks_dir: PathBuf) -> Self { ... }
    
    pub async fn load_all_tasks(&self) -> io::Result<HashMap<TaskId, TaskDisplay>> { ... }
    pub fn save_task(&self, task_id: TaskId, task: &TaskDisplay) -> io::Result<()> { ... }
    pub fn delete_task_file(&self, task_id: TaskId) -> io::Result<()> { ... }
}
```

### **Phase 3: Better Data Structure (Long-term)**

Instead of maintaining both `tasks` HashMap and `task_order` Vec, use a tree structure:

```rust
pub struct TaskNode {
    pub id: TaskId,
    pub data: TaskDisplay,
    pub children: Vec<TaskNode>,
}

pub struct TaskTree {
    roots: Vec<TaskNode>,
    index: HashMap<TaskId, *mut TaskNode>,  // For fast lookup
}

impl TaskTree {
    pub fn add_task(&mut self, task_id: TaskId, task: TaskDisplay, parent_id: Option<TaskId>) {
        let node = TaskNode { id: task_id, data: task, children: Vec::new() };
        
        if let Some(parent_id) = parent_id {
            if let Some(parent_node) = self.index.get(&parent_id) {
                unsafe { (**parent_node).children.push(node); }
            }
        } else {
            self.roots.push(node);
        }
        
        self.rebuild_index();
    }
    
    pub fn get_visible_tasks(&self, collapsed: &HashSet<TaskId>) -> Vec<TaskId> {
        let mut visible = Vec::new();
        for root in &self.roots {
            self.flatten_node(root, collapsed, &mut visible);
        }
        visible
    }
    
    fn flatten_node(&self, node: &TaskNode, collapsed: &HashSet<TaskId>, output: &mut Vec<TaskId>) {
        output.push(node.id);
        
        if !collapsed.contains(&node.id) {
            for child in &node.children {
                self.flatten_node(child, collapsed, output);
            }
        }
    }
}
```

## Implementation Priority

### **Immediate (Phase 1) - Critical Bug Fix**
- [x] Add `rebuild_task_order()` method
- [x] Add `add_task_and_descendants()` helper
- [x] Fix `load_tasks_async()` to rebuild hierarchy
- [x] Fix `TaskStarted` handler to use same logic
- [x] Test: Add tasks, restart app, verify order maintained

### **Medium-term (Phase 2) - Refactoring**
- [x] Create `task_manager.rs` module
- [x] Move task ordering logic to TaskManager
- [x] Create `event_handler.rs` module
- [x] Create `renderer.rs` module
- [x] Create `persistence.rs` module
- [x] Create `state.rs` module
- [x] Create `input.rs` module
- [x] Create `theme.rs` module
- [x] Split `app.rs` from `mod.rs`
- [x] Update all imports and tests

### **Long-term (Phase 3) - Better Architecture**
- [ ] Design TaskTree structure
- [ ] Implement TaskTree with proper tree operations
- [ ] Migrate from HashMap+Vec to TaskTree
- [ ] Add comprehensive tests for tree operations
- [ ] Benchmark performance

## Testing Strategy

### **Unit Tests**
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_hierarchical_order_preserved_on_reload() {
        let mut manager = TaskManager::new();
        
        // Add tasks with hierarchy
        let task1 = create_task("Task 1", None, Instant::now());
        let child_a = create_task("Child A", Some(task1.id), Instant::now() + Duration::from_secs(1));
        let child_b = create_task("Child B", Some(task1.id), Instant::now() + Duration::from_secs(2));
        let task2 = create_task("Task 2", None, Instant::now() + Duration::from_secs(3));
        
        manager.add_task(task1.id, task1);
        manager.add_task(child_a.id, child_a);
        manager.add_task(child_b.id, child_b);
        manager.add_task(task2.id, task2);
        
        let order_before = manager.task_order().to_vec();
        
        // Simulate reload: extract tasks, clear, rebuild
        let tasks = manager.extract_all_tasks();
        manager.clear();
        for (id, task) in tasks {
            manager.tasks.insert(id, task);
        }
        manager.rebuild_order();
        
        let order_after = manager.task_order().to_vec();
        
        assert_eq!(order_before, order_after, "Order should be preserved after rebuild");
        assert_eq!(order_after, vec![task1.id, child_a.id, child_b.id, task2.id]);
    }
    
    #[test]
    fn test_children_always_after_parent() {
        // Test that children are always positioned after parent
        // even when added in different order
    }
    
    #[test]
    fn test_orphaned_tasks_handled() {
        // Test that tasks with non-existent parents are handled gracefully
    }
    
    #[test]
    fn test_deep_nesting() {
        // Test 5+ levels of nesting
    }
}
```

### **Integration Tests**
- Test save/load cycle preserves hierarchy
- Test adding child to middle of tree
- Test deleting parent deletes children
- Test collapsing/expanding maintains order

## Migration Path

1. **Week 1**: Implement Phase 1 fix in current file
2. **Week 2**: Add comprehensive tests for task ordering
3. **Week 3**: Create `task_manager.rs` and migrate logic
4. **Week 4**: Create other modules and migrate code
5. **Week 5**: Refactor main loop and cleanup
6. **Week 6**: Testing and bug fixes

## Benefits of Refactoring

### **Maintainability**
- Each module has clear responsibility
- Easy to locate and fix bugs
- Easier for new contributors

### **Testability**
- TaskManager can be tested independently
- Mock task data easily
- Unit tests for each module

### **Correctness**
- Single source of truth for task ordering
- Hierarchy maintained by design
- Impossible to get out of sync

### **Performance**
- Tree structure is O(1) for lookups
- No rebuilding entire order for each operation
- Better cache locality

## Key Principles

1. **Single Responsibility**: Each module does one thing well
2. **Single Source of Truth**: TaskManager owns the hierarchy
3. **Immutability Where Possible**: Reduce state mutations
4. **Test-Driven**: Write tests first, then implement
5. **Incremental Migration**: Don't rewrite everything at once
