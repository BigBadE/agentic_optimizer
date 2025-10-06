# Clean Tree Output Structure

## Problem Fixed

The output was disjointed with text appearing all over the place because:
1. Initial "Processing..." messages were added at task start
2. Text was being added at wrong nesting levels
3. Multiple code paths were adding output inconsistently

## Clean Structure

### **Task Start**
```
User prompt
  (empty tree)
```

### **During Analysis**
```
User prompt
├─ Analysis
│   └─ {JSON assessment response}
```

### **After Analysis Completes**
```
User prompt
├─ Analysis [+]  ← Auto-collapsed
└─ (waiting for output)
```

### **Final Output**
```
User prompt
├─ Analysis [+]
└─ Hi! How can I help you today?
```

## Implementation

### **1. Clean Task Start** (`ui/mod.rs`)
```rust
UiEvent::TaskStarted { task_id, description, parent_id } => {
    // Start with clean tree - no initial messages
    let output_tree = OutputTree::new();
    // ...
}
```

**Removed**:
- `[i] Task started: ...`
- `[...] Processing...`

### **2. Proper Nesting** (`agent/executor.rs`)

**Analysis Step**:
```rust
// 1. Start analysis step
ui_channel.send(UiEvent::TaskStepStarted {
    task_id,
    step_id: "analysis".to_string(),
    step_type: "Thinking".to_string(),
    content: "Analyzing...".to_string(),
});

// 2. Generate assessment (pushes "analysis" onto stack)
let assessment_response = provider.generate(&query, &context).await?;

// 3. Send output (will be nested under "analysis")
ui_channel.send(UiEvent::TaskOutput {
    task_id,
    output: assessment_response.text.clone(),
});

// 4. Complete analysis (pops from stack, auto-collapses)
ui_channel.send(UiEvent::TaskStepCompleted {
    task_id,
    step_id: "analysis".to_string(),
});
```

**Final Output**:
```rust
// 5. Send final output (stack is empty, goes to root)
ui_channel.send(UiEvent::TaskOutput {
    task_id,
    output: result.clone(),
});
```

### **3. Auto-Nesting Logic** (`ui/output_tree.rs`)

```rust
pub fn add_text(&mut self, content: String) {
    let level = self.current_step_stack.len();
    let node = OutputNode::Text { content, level };
    
    if let Some(parent_id) = self.current_step_stack.last().cloned() {
        // Add as child to current step
        if let Some(parent) = self.find_node_mut(&parent_id) {
            if let OutputNode::Step { children, .. } = parent {
                children.push(node);
            }
        }
    } else {
        // Add to root
        self.root.push(node);
    }
}
```

**Key**: The `current_step_stack` tracks nesting automatically:
- When step starts → Push to stack
- When step completes → Pop from stack
- Text always goes to correct level based on stack

### **4. Auto-Collapse** (`ui/output_tree.rs`)

```rust
pub fn complete_step(&mut self, step_id: &str) {
    if self.current_step_stack.last().map(|s| s.as_str()) == Some(step_id) {
        self.current_step_stack.pop();
    }
    
    // Auto-collapse "analysis" steps when they complete
    if step_id == "analysis" {
        self.collapsed_nodes.insert(step_id.to_string());
    }
}
```

## Expected Output

### **Simple Request: "say hi"**

**While analyzing**:
```
Say hi
└─ Analysis
    └─ Analyzing...
```

**After analysis**:
```
Say hi
├─ Analysis [+]
└─ (generating response...)
```

**Complete**:
```
Say hi
├─ Analysis [+]
└─ Hi! How can I help you today?
```

**When expanded** (press Enter on Analysis):
```
Say hi
├─ Analysis [-]
│   └─ {
│       "action": "COMPLETE",
│       "reasoning": "Simple greeting",
│       "confidence": 0.95,
│       "details": {
│         "result": "Hi! How can I help you today?"
│       }
│     }
└─ Hi! How can I help you today?
```

### **Multiple Tasks**

```
Task 1: Say hi
├─ Analysis [+]
└─ Hi!

Task 2: Explain Rust
├─ Analysis [+]
└─ Rust is a systems programming language...

Task 3: Fix bug
├─ Analysis [+]
└─ Fixed in parser.rs
```

## Key Principles

1. **No premature output** - Don't add messages until there's actual content
2. **Automatic nesting** - Stack-based nesting handles hierarchy
3. **Auto-collapse** - Analysis collapses when complete
4. **Clean separation** - Analysis vs Output are distinct nodes
5. **Single source of truth** - `current_step_stack` determines nesting level

## Files Modified

✅ **`ui/mod.rs`**
- Removed initial status messages from TaskStarted
- Simplified TaskOutput handling (just call add_text)

✅ **`ui/output_tree.rs`**
- Made `current_step_stack` public
- Auto-collapse logic in `complete_step()`

✅ **`agent/executor.rs`**
- Proper step lifecycle (start → output → complete)
- Analysis step contains assessment output
- Final output at root level

## Testing

**Compile**: ✅ `cargo check` passes

**To Test**:
1. Close running merlin.exe
2. `cargo build`
3. Run merlin
4. Type "say hi"
5. Observe clean tree structure

## Result

Clean, hierarchical output with:
- No spurious messages
- Proper nesting
- Auto-collapsed analysis
- Easy to navigate
- Consistent structure across all tasks
