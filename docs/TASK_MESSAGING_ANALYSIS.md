# Task Messaging System Analysis

## Current Flow Trace

### What happens when user submits "Say hi":

```
1. CLI receives input
2. orchestrator.analyze_request("Say hi")
   └─> Returns TaskAnalysis with 1 task

3. For task:
   a) CLI sends: TaskStarted { task_id, description, parent_id }
      └─> UI: Creates TaskDisplay with empty output_area
   
   b) CLI calls: orchestrator.execute_task_streaming(task, ui_channel)
   
   c) AgentExecutor.execute_streaming():
      i)   Sends: TaskStepStarted { "Thinking", "Analyzing task..." }
           └─> UI: Inserts "\n💭 Analyzing task..."
      
      ii)  Sends: ThinkingUpdate { "Analyzing task..." }
           └─> UI: Inserts "\n💭 Analyzing task..." [DUPLICATE!]
      
      iii) Executes LLM query → gets response
      
      iv)  Sends: TaskStepStarted { "Output", response.text }
           └─> UI: Inserts "\n📝 {response.text}"
      
      v)   Returns response to CLI
   
   d) CLI sends: TaskCompleted { task_id, result }
      └─> UI: Inserts "\n{result.response.text}" [DUPLICATE!]
```

## Problems Identified

### 1. **Duplicate Content**
- **Problem**: Content appears twice in output
- **Cause 1**: `TaskStepStarted(Thinking)` + `ThinkingUpdate` both insert same text
- **Cause 2**: `TaskStepStarted(Output)` + `TaskCompleted` both insert response.text

### 2. **UI Corruption (Left-Shift)**
- **Problem**: Text shifts left 2 characters, breaking layout
- **Root Cause**: `auto_wrap_output()` called after EVERY event
- **Why it breaks**:
  - Each call recreates the entire TextArea
  - Cursor position calculated from old lines
  - Applied to new wrapped lines
  - Rapid successive calls cause position drift
  - Unicode icons (💭 🔧) are 2 characters wide, causing offset

### 3. **System Overlap**
- **Legacy events**: `TaskOutput`, `TaskCompleted` append full text
- **Streaming events**: `TaskStepStarted`, `ToolCallStarted` append formatted text
- **Result**: Both systems writing to same output_area

## Architecture Issues

### Current: Two Conflicting Systems

```
┌─────────────────────────────────────────┐
│         Legacy System (Old)             │
├─────────────────────────────────────────┤
│ TaskOutput    → Append plain text       │
│ TaskCompleted → Append full response    │
└─────────────────────────────────────────┘
              ↓ Both write to ↓
         ┌────────────────────┐
         │   output_area      │
         └────────────────────┘
              ↑ Both write to ↑
┌─────────────────────────────────────────┐
│        Streaming System (New)           │
├─────────────────────────────────────────┤
│ TaskStepStarted  → Append with icons    │
│ ThinkingUpdate   → Append with icons    │
│ ToolCallStarted  → Append with icons    │
│ ToolCallCompleted→ Append with icons    │
└─────────────────────────────────────────┘
```

## Proposed Solution: Single Unified System

### Option 1: Pure Streaming (Recommended)

**Principle**: Only streaming events write to output. Completion events only update status.

```
Flow:
1. TaskStarted → Create task display (empty output)
2. TaskStepStarted (Thinking) → Append "💭 Thinking..."
3. TaskStepStarted (ToolCall) → Append "🔧 Calling tool: X"
4. ToolCallCompleted → Append "✓ Tool 'X' completed: {summary}"
5. TaskStepStarted (Output) → Append "📝 {response}"
6. TaskCompleted → Update status ONLY (no text append)
```

**Changes Required**:
- ✅ Remove duplicate `ThinkingUpdate` event (redundant with `TaskStepStarted`)
- ✅ Remove text append from `TaskCompleted` handler
- ✅ Remove text append from `TaskOutput` handler (superseded by streaming)
- ✅ `TaskCompleted` only updates: status, end_time, saves task

**Benefits**:
- No duplication (single source of truth)
- Clean separation: streaming = content, completion = status
- Icons show what agent is doing in real-time
- Response already visible when task completes

### Option 2: Legacy Only (Simpler but Less Informative)

**Principle**: Remove all streaming output, only show final result.

```
Flow:
1. TaskStarted → Create task display (empty output)
2. [Streaming events ignored for output]
3. TaskCompleted → Append full response
```

**Changes Required**:
- ❌ Remove output insertion from all streaming event handlers
- ✅ Keep only `TaskCompleted` appending text

**Benefits**:
- Simpler implementation
- No duplication

**Drawbacks**:
- No real-time visibility
- User doesn't see what agent is doing
- Loses value of streaming system

## Recommended Fix: Option 1 (Pure Streaming)

### Implementation Steps:

#### 1. Remove Redundant ThinkingUpdate
```rust
// In executor.rs - DELETE THIS:
ui_channel.send(UiEvent::ThinkingUpdate {
    task_id,
    content: "Analyzing task and planning approach...".to_string(),
});
```

#### 2. Change TaskCompleted Handler
```rust
// In ui/mod.rs:
UiEvent::TaskCompleted { task_id, result } => {
    self.state.active_running_tasks.remove(&task_id);
    if let Some(task) = self.state.tasks.get_mut(&task_id) {
        task.status = TaskStatus::Completed;
        task.end_time = Some(Instant::now());
        // DON'T append text - already shown via TaskStepStarted(Output)
    }
    // Still call auto_wrap once at end
    self.auto_wrap_output(task_id);
    
    if let Some(task) = self.state.tasks.get(&task_id) {
        self.save_task(task_id, task);
    }
}
```

#### 3. Remove ThinkingUpdate Handler Output
```rust
UiEvent::ThinkingUpdate { task_id: _, content: _ } => {
    // Don't append to output - already shown via TaskStepStarted
}
```

#### 4. Optimize auto_wrap_output Calls
Only call after events that actually change content:
- ✅ Keep: TaskStepStarted, ToolCallStarted, ToolCallCompleted
- ❌ Remove: TaskStepCompleted (doesn't add content)
- ✅ Keep: TaskCompleted (final wrap)

### Why This Fixes The Issues:

1. **No Duplication**: Each piece of text added exactly once
   - Thinking text: Only via `TaskStepStarted(Thinking)`
   - Response text: Only via `TaskStepStarted(Output)`
   - Task completion: Status update only

2. **Fewer auto_wrap_output Calls**: 
   - Reduces TextArea recreation
   - Less cursor position drift
   - Better performance

3. **Clean Separation**:
   - Streaming events = content updates
   - Completion events = status updates
   - No overlap, no confusion

4. **Better UX**:
   - See agent thinking in real-time
   - See tool calls as they happen
   - Response incrementally visible
   - Clear what's happening

## Current vs Proposed

### Current (Broken):
```
TaskStepStarted(Thinking) → "💭 Analyzing..."
ThinkingUpdate            → "💭 Analyzing..."  [DUP!]
TaskStepStarted(Output)   → "📝 Hello!"
TaskCompleted             → "Hello!"          [DUP!]
```

### Proposed (Clean):
```
TaskStepStarted(Thinking) → "💭 Analyzing..."
TaskStepStarted(Output)   → "📝 Hello!"
TaskCompleted             → [status update only]
```

## Migration Path

1. Apply fixes in order listed above
2. Test with simple prompt "Say hi"
3. Verify no duplication
4. Verify no UI corruption
5. Test with tool-using prompts
6. Verify tool calls display correctly
