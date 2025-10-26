# Issues and Recommendations

Active issues and improvements needed for the Merlin codebase.

## Summary (2025-10-26)

**Thread system FULLY IMPLEMENTED - All phases complete!**

**Core System (Phases 1-8):**
- ✅ Unified `Subtask` type with optional verification (replaces SubtaskSpec and TaskList)
- ✅ Thread-based conversation history with automatic context extraction
- ✅ ThreadStore persistence (`.merlin/threads/*.json`)
- ✅ Work tracking with completion/failure status
- ✅ Task blocking with cancel/queue support

**UI Implementation (Phases 9-10):**
- ✅ Thread navigation panel with full keyboard support
- ✅ Side-by-side layout (threads | work details)
- ✅ Thread creation, branching, archiving
- ✅ Work cancellation and queueing
- ✅ Thread-based test fixtures

**Keybindings:**
- `Ctrl+Shift+T` - Toggle thread panel focus
- `n` - Create new thread (in thread pane)
- `b` - Branch from current message
- `d` - Archive/delete thread
- `↑↓` - Navigate thread list
- `c`/`a`/`Esc` - Cancel/queue/discard when work is running

---

## Unified Thread-Based Execution System

### System Overview

**Core principle:** Tasks are ephemeral work containers. Threads are persistent conversation contexts.

All work happens within threads:
- Each user message creates or continues a thread
- Threads maintain full conversation history
- Tasks spawn from messages and create work units
- Work units contain subtasks with optional verification
- Thread history automatically provides context to agents

### Mental Model

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

### Implemented Type System

**Core Types:**
- `Thread` - Conversation with messages and context manager
- `ThreadId` - Unique identifier for threads
- `Message` - User input that spawns work
- `WorkUnit` - Unit of work spawned by a message
- `Subtask` - Individual task within work unit (unified type with optional verification)
- `VerificationStep` - Optional verification for subtasks

**Thread Structure:**
- Messages: Vec of user/assistant conversation
- Context: Shared context manager for the thread
- Persistence: Auto-saved to `.merlin/threads/<id>.json`

**Subtask (Unified Type):**
- Replaces both SubtaskSpec (agent decomposition) and TaskList (verification steps)
- Fields: description, difficulty, status, optional verification
- Supports both sequential and parallel execution

**Status Tracking:**
- WorkStatus: Planning, Executing, Verifying, Completed, Failed, Cancelled
- SubtaskStatus: Pending, InProgress, Completed, Failed, Cancelled

---

### TUI Display Design (Implemented - Phase 9)

**Status:** Fully implemented! This section describes the thread navigation UI that is now live.

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

**Phase 4: Thread-Aware Orchestrator** ✅ COMPLETED
- ✅ Add thread storage to RoutingOrchestrator
- ✅ Add `execute_task_in_thread()` method
- ✅ Extract conversation history from threads
- ✅ Add `with_thread_store()` builder method
- ✅ Create `ConversationHistory` type alias

**Phase 5: TUI Thread Panel Foundation** ✅ COMPLETED
- ✅ Add `FocusedPane::Threads` enum variant
- ✅ Add `render_thread_list()` method
- ✅ Add ThreadStore initialization in TuiApp

**Phase 6: ThreadStore CLI Integration** ✅ COMPLETED
- ✅ ThreadStore created in handlers.rs
- ✅ ThreadStore passed to orchestrator
- ✅ Threads loaded on startup from `.merlin/threads/`
- ✅ Thread lifecycle (create, load, save) fully integrated

**Phase 7: Thread-Based Task Execution** ✅ COMPLETED
- ✅ Tasks automatically create/continue threads
- ✅ Messages added to threads when user submits input
- ✅ WorkUnit attached to messages
- ✅ Work completion/failure tracked in thread messages
- ✅ Thread history passed to orchestrator

**Phase 8: Work Queue Management** ✅ COMPLETED (Implicit)
- ✅ Task blocking via single `pending_input` slot
- ✅ Prevents simultaneous task execution
- Note: Explicit queue UI deferred to Phase 9

**Phase 9: Thread Navigation UI** ✅ COMPLETED
- ✅ Implemented side-by-side split (threads 30% | work details 70%)
- ✅ Added thread color emoji display in list
- ✅ Added selection highlighting with `>` indicator
- ✅ Implemented n/b/d keybindings for thread operations
- ✅ Implemented cancel/queue prompt when work running
- ✅ Thread list shows message counts and work status emojis
- ✅ Help text displayed when thread pane focused

**Phase 10: Thread-Based Test Fixtures** ✅ COMPLETED
- ✅ Created `basic_thread_creation.json` - tests thread creation and navigation
- ✅ Created `thread_branch.json` - tests branching from messages
- ✅ Created `work_cancellation.json` - tests cancel/queue workflow
- ✅ Fixtures use standard TUI event format (key_press, user_input)
- ✅ Verification points for thread count, selection state, UI state

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
