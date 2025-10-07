# Task Decomposition Guide

## Problem

Simple user requests like "say hi" were being decomposed into multiple tasks (e.g., 3 tasks all with the same output), creating unnecessary complexity and confusion in the UI.

## Solution

### 1. Temporary Status Output ✅

**Added initial status messages** when a task starts:
```rust
// In TaskStarted event handler
let mut output_tree = OutputTree::new();
output_tree.add_text(format!("[i] Task started: {}", description));
output_tree.add_text("[...] Processing...".to_string());
```

**Benefits**:
- Users see immediate feedback that the task is running
- Clear indication of what's happening before actual output arrives
- Better UX for tasks that take time to process

**Example Output**:
```
  [i] Task started: Respond to greeting
  [...] Processing...
```

Once the task produces actual output, the tree will show:
```
  [i] Task started: Respond to greeting
  [*] Thinking: Analyzing request...
  [>] Output: Hi! How can I help you today?
```

### 2. Task Decomposition Recommendations

The issue of multiple tasks for simple requests is in the **Decomposer** component. Here's how to fix it:

#### Current Behavior (Problematic)
```
User: "say hi"
→ Decomposer creates:
  1. Task: "Analyze greeting request"
  2. Task: "Generate greeting response"
  3. Task: "Format and deliver response"
```

#### Recommended Behavior
```
User: "say hi"
→ Decomposer creates:
  1. Task: "Respond to greeting"  (single task)
```

#### Implementation Guidelines

**Simple Request Detection**:
```rust
fn is_simple_request(input: &str) -> bool {
    let word_count = input.split_whitespace().count();
    let has_complex_keywords = input.contains("analyze") 
        || input.contains("compare")
        || input.contains("refactor")
        || input.contains("implement");
    
    // Simple if: short AND no complex keywords
    word_count <= 5 && !has_complex_keywords
}
```

**Task Decomposition Logic**:
```rust
pub async fn decompose(&self, request: &str) -> Result<Vec<Task>> {
    // Check if this is a simple request
    if is_simple_request(request) {
        // Create single task for simple requests
        return Ok(vec![Task {
            id: TaskId::new(),
            description: request.to_string(),
            complexity: Complexity::Simple,
            dependencies: vec![],
        }]);
    }
    
    // For complex requests, use LLM-based decomposition
    self.llm_decompose(request).await
}
```

**Complexity Levels**:
```rust
pub enum Complexity {
    Simple,      // 1 task: greetings, simple questions
    Moderate,    // 2-3 tasks: code reviews, explanations
    Complex,     // 4+ tasks: refactoring, multi-file changes
}
```

#### Examples

**Simple Requests (1 task)**:
- "say hi"
- "hello"
- "what time is it?"
- "thanks"
- "help"

**Moderate Requests (2-3 tasks)**:
- "explain this function"
- "review my code"
- "fix this bug"
- "add error handling"

**Complex Requests (4+ tasks)**:
- "refactor the entire module"
- "implement a new feature with tests"
- "analyze and optimize performance"
- "migrate to new framework"

### 3. Decomposer Location

The Decomposer is likely in one of these locations:
- `crates/merlin-routing/src/decomposer.rs`
- `crates/merlin-routing/src/intent/decomposer.rs`
- `crates/merlin-agent/src/decomposer.rs`

**To fix**:
1. Find the `Decomposer` struct
2. Add `is_simple_request()` helper
3. Update `decompose()` method to check complexity first
4. Return single task for simple requests

### 4. Alternative: Intent-Based Routing

Instead of always decomposing, route based on intent:

```rust
pub enum Intent {
    Greeting,           // "hi", "hello" → single response task
    Question,           // "what is X?" → single answer task
    CodeReview,         // "review code" → 2-3 tasks
    Implementation,     // "implement X" → 3-5 tasks
    Refactoring,        // "refactor X" → 4+ tasks
}

pub async fn route_request(&self, request: &str) -> Result<Vec<Task>> {
    let intent = self.extract_intent(request).await?;
    
    match intent {
        Intent::Greeting | Intent::Question => {
            // Single task for simple intents
            vec![Task::simple(request)]
        }
        Intent::CodeReview => {
            // 2-3 tasks for moderate complexity
            self.decompose_code_review(request).await?
        }
        Intent::Implementation | Intent::Refactoring => {
            // Full decomposition for complex work
            self.decompose_complex(request).await?
        }
    }
}
```

### 5. UI Improvements

**Status Indicators**:
- `[i]` - Info/Status message
- `[...]` - Processing/Waiting
- `[*]` - Thinking
- `[T]` - Tool call
- `[>]` - Output
- `[+]` - Success
- `[X]` - Error

**Tree Structure**:
```
Task: Respond to greeting
  [i] Task started: Respond to greeting
  [...] Processing...
  [*] Thinking: Analyzing greeting...
  [>] Output: Hi! How can I help you today?
```

vs. **Over-decomposed** (bad):
```
Task: Analyze greeting request
  [i] Task started: Analyze greeting request
  [>] Output: User said "hi"

Task: Generate greeting response
  [i] Task started: Generate greeting response
  [>] Output: Response: "Hi!"

Task: Format and deliver response
  [i] Task started: Format and deliver response
  [>] Output: Hi! How can I help you today?
```

## Summary

### ✅ Completed
- Added temporary status output (`[i]` and `[...]`) when tasks start
- Users now see immediate feedback before actual output arrives

### 📝 Recommended Next Steps
1. **Find the Decomposer** component
2. **Add simple request detection** to avoid over-decomposition
3. **Implement intent-based routing** for better task creation
4. **Test with simple requests** like "say hi", "hello", "thanks"

### Expected Behavior After Fix
- **"say hi"** → 1 task: "Respond to greeting"
- **"fix this bug"** → 2-3 tasks: "Analyze bug", "Implement fix", "Test fix"
- **"refactor module"** → 4+ tasks: "Analyze structure", "Plan refactoring", "Implement changes", "Update tests", "Verify functionality"

The key is **matching task count to request complexity**, not creating tasks for the sake of having tasks.
