# Issues and Recommendations

Active issues and improvements needed for the Merlin codebase.

## Status Update (2025-10-26)

**PHASES 1-7 COMPLETED:**
- ✅ Thread type system fully implemented and tested
- ✅ Old types (SubtaskSpec, TaskList) completely removed
- ✅ TaskListExecutor deleted
- ✅ UI parent-child hierarchy removed (flat task system)
- ✅ Thread-aware orchestrator implemented
- ✅ Thread panel rendering with color-coded threads and status emojis
- ✅ **ThreadStore integrated into CLI lifecycle** (Phase 6)
- ✅ **Threads automatically created for every task** (Phase 7)
- ✅ **Messages added to threads when user submits input** (Phase 7)
- ✅ **WorkUnit attached to messages with completion status** (Phase 7)
- ✅ **Threads saved to disk with full work history** (Phase 7)
- ✅ All 290 tests passing
- ✅ All 53 fixtures passing

**Key Changes:**
- Unified `Subtask` type replaces both `SubtaskSpec` and `TaskList`
- `TaskAction::Decompose` now uses `Vec<Subtask>`
- `TaskDisplay.parent_id` replaced with `TaskDisplay.thread_id`
- `TaskManager` simplified to chronological ordering
- Backward compatibility via serde defaults for `id` and `status` fields
- `RoutingOrchestrator` now has optional `ThreadStore` support
- New `execute_task_in_thread()` method with automatic history extraction
- `ConversationHistory` type alias for cleaner APIs
- `FocusedPane::Threads` enum variant for thread panel navigation
- `render_thread_list()` method displays threads with color emojis and work status
- **ThreadStore created in `handlers.rs` and passed to orchestrator**
- **Tasks automatically create/continue threads with thread-aware execution**
- **Threads loaded on startup from `.merlin/threads/` directory**
- **Work completion/failure tracked in thread messages**

**Implementation Notes:**
- Thread panel ready for integration (currently has placeholder dead_code allows)
- ThreadStore lifecycle fully integrated (create, load, save)
- All tasks now execute within thread context
- Thread history automatically passed to orchestrator for context
- Threads persist to disk as JSON in `.merlin/threads/<thread_id>.json`
- **Phase 8**: Basic work queue management implemented via task blocking (prevents simultaneous tasks)
- **Phase 9**: Thread navigation UI deferred (requires complex TUI changes, not critical for core functionality)
- **Phase 10**: Thread-based fixtures in progress
- Core thread system fully functional and integrated

---

## Unified Thread-Based Execution System

### Current System: Three Separate Concepts

**1. Subtasks (Self-Assessment Decomposition)**
- Agent decides to decompose task via `TaskAction::Decompose`
- Creates `SubtaskSpec` instances with descriptions and difficulty
- Executed sequentially or in parallel via `execute_with_subtasks()`
- **Not visible in TUI** - just shows "Decomposing into N subtasks" message
- Results combined into single response

**2. TaskList (TypeScript-Determined Steps)**
- Agent writes TypeScript code that creates a `TaskList`
- Steps have: id, type (Debug/Feature/Test), description, verification, exit_command
- Executed sequentially via `TaskListExecutor`
- **Visible in TUI** as sub-items under parent task
- Each step has status: Pending → InProgress → Completed/Failed

**3. Parent-Child Tasks (TUI Conversation Continuity)**
- User presses 'c' to continue conversation
- Creates new task with `parent_id` set
- Normalized to max depth 1 (no grandchildren)
- **Displayed as indented tasks**
- Used to extract conversation history

**Problem:** These three systems overlap and conflict. We need ONE unified model.

---

### Proposed: Unified Thread System

**Core Insight:** Everything is a **Message** in a **Thread**, and messages can spawn **Work** (tasks with subtasks).

#### Mental Model

```
Thread = Conversation
├─ Message 1 (User): "Add authentication"
│  └─ Work spawned: Task "Add authentication"
│     ├─ Subtask 1: "Create User model"
│     ├─ Subtask 2: "Add login endpoint"
│     └─ Subtask 3: "Write integration tests"
├─ Message 2 (User): "Add password reset"
│  └─ Work spawned: Task "Add password reset"
│     ├─ Subtask 1: "Create reset token model"
│     └─ Subtask 2: "Add reset endpoint"
```

**Key principle:** Tasks are ephemeral work containers. Threads are persistent conversation contexts.

---

### Type System Design

**Core Types:**
- `Thread` - Conversation with messages and context manager
- `ThreadId` - Unique identifier for threads
- `Message` - User input that spawns work
- `WorkUnit` - Unit of work spawned by a message
- `WorkId` - Unique identifier for work
- `Subtask` - Individual task within work unit (merges SubtaskSpec and TaskList)
- `SubtaskId` - Unique identifier for subtasks
- `VerificationStep` - Optional verification for subtasks (replaces TaskList)

**Message:**
- User input + timestamp + WorkUnit

**WorkUnit:**
- Description, subtasks, execution mode (sequential/parallel), status

**Subtask:**
- Description, difficulty, status, optional verification step
- Merges both agent decomposition AND TaskList into one concept

**WorkStatus:**
- Planning, Executing, Verifying, Completed, Failed, Cancelled

**SubtaskStatus:**
- Pending, InProgress, Completed, Failed, Cancelled

---

### TUI Display Design

#### Main View: Side-by-Side Thread List + Work Details

**Color Scheme:**
- User messages: Blue text
- Thread color dots: 🔵 🟢 🟣 🟡 🔴 (for visual grouping)
- Status emojis: ⏳ ✅ ❌ ⏸️ (in-progress, done, failed, paused)

```
┌─ Threads (4) ─────────────────────────┬─ Work: Add authentication ────────────┐
│                                        │                                        │
│ 🔵 Authentication work        (4 msgs)│ Status: In Progress (2/3 completed)   │
│   Add authentication                   │ Mode: Sequential                       │
│     ✅ Create User model                │                                        │
│     ✅ Add login endpoint               │ ✅ Create User model                   │
│     ⏳ Write integration tests          │    Verified: cargo check               │
│   Add password reset                   │                                        │
│     ⏸️ Planning...                      │ ✅ Add login endpoint                  │
│                                        │    Verified: cargo test test_login     │
│ 🟢 Database refactor          (2 msgs)│                                        │
│   Refactor database layer              │ ⏳ Write integration tests              │
│     ✅ Refactor connection pooling      │    ✅ Test successful login            │
│                                        │       Output: test ... ok              │
│ 🟣 Documentation               (1 msg) │    ⏳ Test login failure    [running]  │
│   Update README                        │       Output: Compiling auth...        │
│     ⏳ Update README              [1/2]│    ⏸️ Test token expiration            │
│                                        │                                        │
│ 🟡 OAuth Integration      (branched)   │                                        │
│   ↳ from: Authentication > msg 1       │ [Select thread/message to view work]  │
│   Use OAuth instead                    │                                        │
│     ⏳ Setting up OAuth flow            │                                        │
│                                        │                                        │
│ > _                                    │                                        │
└────────────────────────────────────────┴────────────────────────────────────────┘
Keys: ↑↓=navigate  Enter=send  n=new thread  b=branch  Esc=cancel work
      Space=expand/collapse  /=search  ?=help
```

**Thread Colors** - Assigned automatically for visual distinction:
- 🔵 Blue
- 🟢 Green
- 🟣 Purple
- 🟡 Yellow
- 🔴 Red
- 🟠 Orange

**Status Emojis:**
- ⏳ - In progress
- ✅ - Completed
- ❌ - Failed
- ⏸️ - Pending/Paused
- 🔄 - Planning/Thinking

---

#### Sending Messages While Work In Progress

When user sends a message while work is running, show options:

```
┌─ Threads ─────────────────────────────┬─ Work: Add authentication ────────────┐
│                                        │                                        │
│ 🔵 Authentication work                 │ ⏳ Write integration tests              │
│   Add authentication                   │    ✅ Test successful login            │
│   Add password reset                   │    ⏳ Test login failure    [running]  │
│                                        │    ⏸️ Test token expiration            │
│ ┌────────────────────────────────────┐│                                        │
│ │ Work in progress. Send message to: ││                                        │
│ │                                    ││                                        │
│ │ [c] Cancel current work and start  ││                                        │
│ │ [a] Add to queue (run after)       ││                                        │
│ │ [Esc] Go back                      ││                                        │
│ └────────────────────────────────────┘│                                        │
│                                        │                                        │
│ > Fix the validation bug_              │                                        │
└────────────────────────────────────────┴────────────────────────────────────────┘
```

**Cancel (c):**
- Stops current work immediately
- Marks in-progress subtasks as cancelled
- Starts new work from user's message

**Add to queue (a):**
- Lets current work finish
- Queues new message as next work item
- Shows queued indicator in thread list

```
│ 🔵 Authentication work                 │
│   Add authentication                   │
│     ✅ Create User model                │
│   Add password reset                   │
│     ⏸️ Planning...                      │
│   Fix validation bug            [queue]│
```

---

#### Collapsed/Expanded States

**Collapsed (default for completed/inactive work):**
```
│ 🔵 Authentication work        (4 msgs)│
│   Add authentication                   │
│     Work: ✅✅✅ [3 tasks]              │
│   Add password reset                   │
│     ⏸️ Planning...                      │
```

**Expanded (Space to toggle):**
```
│ 🔵 Authentication work        (4 msgs)│
│   Add authentication                   │
│     ✅ Create User model                │
│     ✅ Add login endpoint               │
│     ✅ Write integration tests          │
│   Add password reset                   │
│     ⏸️ Planning...                      │
```

**Auto-expand rules:**
- Currently running work: Always expanded
- Failed work: Always expanded
- Completed work: Collapsed after 30 seconds
- User can override with Space

---

#### Branch Visualization

**In Thread List:**
```
┌─ Threads ─────────────────────────────┬─ Work Details ────────────────────────┐
│                                        │                                        │
│ 🔵 Authentication work        (4 msgs)│                                        │
│   Add authentication                   │                                        │
│     ✅ Create User model                │                                        │
│     ✅ Add login endpoint               │                                        │
│     ✅ Write integration tests          │                                        │
│   Add password reset                   │                                        │
│     ✅ Reset token model                │                                        │
│                                        │                                        │
│ 🟡 OAuth Integration      (branched)   │                                        │
│   ↳ from: Authentication > msg 1       │ Branch Info:                           │
│   Use OAuth instead                    │ Parent: 🔵 Authentication work         │
│     ⏳ Set up OAuth providers           │ Branched from: "Add authentication"   │
│     ⏸️ Configure callback URLs          │ History: 1 message carried over       │
│                                        │                                        │
│ 🟢 Sessions               (branched)   │                                        │
│   ↳ from: Authentication > msg 1       │                                        │
│   Add session management               │                                        │
│     ⏳ Create session store             │                                        │
│                                        │                                        │
└────────────────────────────────────────┴────────────────────────────────────────┘
```

Shows visual connection with `↳` and parent thread color/name.

---

#### Alternate Layout: Vertical Split for Narrow Terminals

```
┌─ Threads (4) ──────────────────────────────────────────────────┐
│                                                                 │
│ 🔵 Authentication work                             (4 messages)│
│   Add authentication                                            │
│     ✅ Create User model                                        │
│     ✅ Add login endpoint                                       │
│     ⏳ Write integration tests                          [2/3]   │
│   Add password reset                                            │
│     ⏸️ Planning...                                              │
│                                                                 │
│ 🟢 Database refactor                               (2 messages)│
│   Refactor database layer                                       │
│     ✅ Refactor connection pooling                              │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ Work: Write integration tests                                  │
│                                                                 │
│ ✅ Test successful login                                        │
│    Verified: cargo test test_login_success                     │
│                                                                 │
│ ⏳ Test login failure                               [running]   │
│    Output: Compiling auth module...                            │
│            Running tests...                                    │
│                                                                 │
│ ⏸️ Test token expiration                                        │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

### User Interaction Flows

#### 1. Starting a New Thread

```
Action: Press 'n'
View: Input prompt appears at bottom of thread list
User: "Add authentication"_

→ Creates new thread with random color (🔵)
→ Adds thread to list
→ Executes task with empty history
→ Shows planning status (🔄)
→ Agent decomposes into subtasks
→ Begins executing, switches to ⏳
```

#### 2. Replying in Same Thread

```
Action: Select thread, type message
User: "Add password reset"_

→ Gets thread context (all previous messages)
→ Adds message to thread
→ If previous work still running:
    → Show [c]ancel / [a]dd to queue prompt
→ Otherwise:
    → Execute immediately with full history
→ Shows in work details panel
```

#### 3. Branching from Message

```
Action: Navigate to message, press 'b'
View: Input prompt shows "branching from: Add authentication"
User: "Use OAuth instead"_

→ Creates new thread with different color (🟡)
→ Shows branch indicator: ↳ from: Authentication > msg 1
→ Copies history up to selected message (excludes responses)
→ Executes with partial context
→ Work details show parent thread info
```

#### 4. Canceling Running Work

```
Action: Press Esc while work is running
→ Immediately cancels current work
→ Marks in-progress subtasks with ❌ Cancelled
→ Thread ready for new message
→ Previous work still visible in history

OR

Action: Send new message while work running, press 'c'
→ Cancels current work
→ Starts new work from message
```

#### 5. Queuing Work

```
Action: Send message while work running, press 'a'

Display:
│ 🔵 Authentication work                 │
│   Add authentication                   │
│     ⏳ Create User model    [running]  │
│   Add password reset          [queue]  │
│   Fix validation              [queue]  │

→ Current work continues
→ Queued messages show [queue] indicator
→ Execute in order when previous completes
→ Can cancel queue with Backspace on queued item
```

---

### Alternative Timeline Views

**Option 1: Minimalist (default)**
Current design - side-by-side with thread list and work details

**Option 2: Chat-style Drill-down**
Press Enter on thread to expand into full-screen chat view:

```
┌─ 🔵 Authentication work ────────────────────────────────────────┐
│                                                                 │
│ Add authentication                                              │
│ ├─ ✅ Create User model                                         │
│ ├─ ✅ Add login endpoint                                        │
│ └─ ⏳ Write integration tests                          [2/3]    │
│     ├─ ✅ Test successful login                                 │
│     ├─ ⏳ Test login failure                         [running]  │
│     └─ ⏸️ Test token expiration                                 │
│                                                                 │
│ Add password reset                                              │
│ └─ ⏸️ Planning...                                               │
│                                                                 │
│ > _                                                             │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
Keys: Esc=back to list  b=branch from selected message
```

**Option 3: Compact Overview**
Press Space to collapse all work, show only summary:

```
┌─ Threads ───────────────────────────────────────────────────────┐
│                                                                 │
│ 🔵 Authentication work (4 msgs) ⏳                               │
│    ✅✅⏳⏸️                                                       │
│                                                                 │
│ 🟢 Database refactor (2 msgs) ✅                                │
│    ✅                                                           │
│                                                                 │
│ 🟣 Documentation (1 msg) ⏳                                      │
│    ⏳                                                           │
│                                                                 │
│ 🟡 OAuth Integration (branched) ⏳                               │
│    ↳ Auth  ⏳⏸️                                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

### Visual Design Details

**Color Usage:**

1. **Thread identification** - Emoji dots (🔵🟢🟣🟡🔴🟠)
2. **User messages** - Blue text
3. **Status** - Emojis (⏳✅❌⏸️🔄)
4. **Branches** - Show parent color in indicator

**Typography Hierarchy:**
- Thread titles: Bold
- Messages: Regular weight, colored by role
- Work items: Indented with tree chars (├─└─│)
- Status indicators: Emoji + text

**Information Density:**
- Default: Show 2-3 levels (threads → messages → subtasks)
- Collapsed: 1 level (threads → summary)
- Expanded: 4 levels (threads → messages → subtasks → verification)
- Drill-down: Full detail (threads → messages → subtasks → output → logs)

**Animations (if supported):**
- ⏳ spinner rotates for in-progress items
- New messages slide in from bottom
- Completed work fades to collapsed state after 30s
- Failed work pulses red briefly

---

### Keybindings (Final)

**Global:**
- `↑/↓` - Navigate threads/messages/subtasks
- `Enter` - Send message in selected thread / drill into item
- `Esc` - Cancel work / go back up one level
- `Ctrl+c` - Force quit work (hard cancel)
- `Space` - Expand/collapse selected item
- `/` - Search threads
- `?` - Help

**Thread List:**
- `n` - New thread
- `b` - Branch from selected message
- `f` - Fork entire thread
- `Backspace/Del` - Delete thread or queued message

**During Input:**
- `c` - Cancel running work (when prompted)
- `a` - Add to queue (when prompted)
- `Enter` - Send message
- `Esc` - Cancel input

**Work Details:**
- `r` - Retry failed subtask
- `l` - View full logs
- `Space` - Expand/collapse output

---

### Implementation Phases

**Phase 1: Core Thread Types (2 days)** ✅ COMPLETED
- ✅ Add Thread, Message, WorkUnit, Subtask types to merlin-core
- ✅ Merge SubtaskSpec and TaskList into unified Subtask with optional verification
- ✅ Add ThreadStore for persistence
- ✅ Write unit tests
- ✅ Add serde defaults for backward compatibility (id, status fields)

**Phase 2: Old Types Removal (1 day)** ✅ COMPLETED
- ✅ Remove SubtaskSpec type (replaced with Subtask)
- ✅ Remove TaskList type (replaced with Subtask)
- ✅ Remove TaskListExecutor module
- ✅ Update TaskAction::Decompose to use Vec<Subtask>
- ✅ Remove all TaskList-related code from executor and orchestrator
- ✅ Update all imports and references
- ✅ All tests passing (290/290)
- ✅ All fixtures passing (53/53)

**Phase 3: TUI Parent-Child Removal (1 day)** ✅ COMPLETED
- ✅ Remove parent_id from TaskDisplay (replaced with thread_id)
- ✅ Update all UI code to remove hierarchical task logic
- ✅ Simplify TaskManager to flat task ordering
- ✅ Update event handlers to work without parent_id
- ✅ Update persistence to save thread_id instead
- ✅ All UI tests passing

**Phase 4: Thread-Aware Orchestrator (3 days)** ✅ COMPLETED
- ✅ Add thread storage to RoutingOrchestrator
- ✅ Add `execute_task_in_thread()` method
- ✅ Extract conversation history from threads
- ✅ Add `with_thread_store()` builder method
- ✅ Create `ConversationHistory` type alias
- ⏸️ Queue management (deferred to Phase 6)

**Phase 5: TUI Side-by-Side Layout (3 days)** ⏸️ PENDING
- Implement side-by-side split (threads | work details)
- Add thread color assignment
- Add expand/collapse state management
- Update persistence to store threads
- Add ThreadStore initialization (already in TuiApp)

**Phase 6: User Input & Work Control (2 days)** ⏸️ PENDING
- Update input handling for thread context
- Implement cancel/queue prompt when work running
- Add n/b/f keybindings
- Add thread selection and navigation
- Add work cancellation logic

**Phase 7: Work Unit Display (2 days)** ⏸️ PENDING
- Implement subtask progress display with emojis
- Add verification step visualization
- Handle parallel vs sequential display
- Add drill-down views
- Add status animations

**Phase 8: Testing & Polish (2 days)** ⏸️ PENDING
- Create fixtures for threaded conversations
- Test subtask execution with verification
- Test branching/forking scenarios
- Test cancel/queue behavior
- Performance testing
- Color contrast testing

**Total: ~14 days**

---

## Future Enhancements

### merlin-languages
1. Add fixture coverage for language analysis scenarios
2. Add support for more languages (TypeScript, Python, Go)
3. Implement caching for language server results

### merlin-local
1. Add mock Ollama API server for more comprehensive testing
2. Add fixture coverage for local model execution scenarios
3. Add performance benchmarks for inference latency
