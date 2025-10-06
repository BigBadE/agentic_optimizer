# Tree Output Implementation - Collapsible Analysis

## Changes Made

### **1. Tree Structure**

**Before**:
```
User prompt
  [i] Task started
  [...] Processing...
  [*] Assessing: Analyzing...
  [>] Decision: COMPLETE
  Output text
```

**After**:
```
User prompt
├─ Analysis [collapsed]
└─ Output text
```

**When expanded** (press Enter on "Analysis"):
```
User prompt
├─ Analysis
│   └─ (Raw token output from assessment)
└─ Output text
```

### **2. Auto-Collapse Behavior**

**Implementation** (`output_tree.rs`):
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

**Behavior**:
- When "Analysis" step completes, it's automatically collapsed
- User can expand it by pressing Enter/Space on the node
- Multiple tasks each have their own collapsed Analysis section

### **3. Execution Flow**

**Updated** (`agent/executor.rs`):

1. **Start Analysis Step**:
```rust
ui_channel.send(UiEvent::TaskStepStarted {
    task_id,
    step_id: "analysis".to_string(),
    step_type: "Thinking".to_string(),
    content: "Analyzing...".to_string(),
});
```

2. **Capture Raw Assessment Output**:
```rust
let assessment_response = provider.generate(&query, &context).await?;

// Send raw output to UI (will be under "Analysis" step)
ui_channel.send(UiEvent::TaskOutput {
    task_id,
    output: assessment_response.text.clone(),
});
```

3. **Complete Analysis (Auto-Collapse)**:
```rust
ui_channel.send(UiEvent::TaskStepCompleted {
    task_id,
    step_id: "analysis".to_string(),
});
// This triggers auto-collapse in output_tree
```

4. **Add Final Output**:
```rust
ui_channel.send(UiEvent::TaskOutput {
    task_id,
    output: result.clone(),
});
```

### **4. Multiple Tasks**

Each task gets its own tree structure:

```
Task 1: Say hi
├─ Analysis [collapsed]
└─ Hi! How can I help you?

Task 2: Explain Rust
├─ Analysis [collapsed]
└─ Rust is a systems programming language...

Task 3: Fix bug
├─ Analysis [collapsed]
└─ Fixed the issue in parser.rs
```

**Navigation**:
- Use arrow keys to move between tasks
- Press Enter/Space on "Analysis" to expand/collapse
- Each task's analysis is independent

## Files Modified

✅ **`crates/merlin-routing/src/agent/executor.rs`**
- Changed step name from "assess" to "analysis"
- Capture raw assessment response
- Send token output to UI before parsing
- Auto-collapse after completion

✅ **`crates/merlin-routing/src/ui/output_tree.rs`**
- Added auto-collapse logic in `complete_step()`
- Collapses nodes with id "analysis" automatically

✅ **`crates/merlin-routing/src/agent/self_assess.rs`**
- Exposed `parse_assessment_response()` as public method
- Allows executor to parse response after capturing it

## User Experience

### **While Running**:
```
User prompt
└─ Analysis
    └─ Analyzing...
```

### **After Completion**:
```
User prompt
├─ Analysis [+]  ← Collapsed, press Enter to expand
└─ Hi! How can I help you today?
```

### **When Expanded**:
```
User prompt
├─ Analysis [-]  ← Expanded, press Enter to collapse
│   └─ {
│       "action": "COMPLETE",
│       "reasoning": "Simple greeting, can respond immediately",
│       "confidence": 0.95,
│       "details": {
│         "result": "Hi! How can I help you today?"
│       }
│     }
└─ Hi! How can I help you today?
```

## Benefits

✅ **Clean UI** - Analysis hidden by default  
✅ **Inspectable** - Can expand to see reasoning  
✅ **Multiple tasks** - Each has its own collapsed section  
✅ **Automatic** - No manual collapse needed  
✅ **Consistent** - Same behavior for all tasks  

## Testing

**Compile Status**: ✅ Passes `cargo check`

**To Test**:
1. Close any running `merlin.exe` process
2. Run `cargo build`
3. Start merlin in TUI mode
4. Type "say hi"
5. Observe:
   - "Analysis" appears and auto-collapses
   - Output appears below
   - Press Enter on "Analysis" to expand/collapse

## Future Enhancements

- Add icons for collapsed/expanded state (`[+]` / `[-]`)
- Color-code analysis vs output
- Add timing information to analysis node
- Support nested analysis for decomposed tasks
