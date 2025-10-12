# E2E Test Scenario Schema

## Overview

Unified JSON format for comprehensive end-to-end testing that covers:
- Agent responses and tool calls
- Task spawning and hierarchy
- Background tasks (embedding cache, etc.)
- UI state verification with snapshots
- User input simulation
- Event verification

## Schema

```json
{
  "name": "Test Scenario Name",
  "description": "What this scenario tests",
  "config": {
    "terminal_size": [80, 30],
    "enable_vector_cache": true,
    "mock_embedding_speed": 100
  },
  "initial_state": {
    "workspace_files": [
      {"path": "src/main.rs", "content": "fn main() {}"},
      {"path": "config.json", "content": "{...}"}
    ],
    "existing_tasks": [],
    "vector_cache_state": "empty" | "partial" | "complete"
  },
  "steps": [
    {
      "step": 1,
      "action": {
        "type": "user_input" | "agent_response" | "tool_result" | "wait" | "background_event",
        "data": {...}
      },
      "expectations": {
        "tasks": {
          "spawned": [...],
          "completed": [...],
          "no_unexpected": true
        },
        "background_tasks": {
          "embedding_cache": {
            "status": "running" | "completed",
            "progress": 42
          }
        },
        "ui_state": {
          "snapshot": "scenario_name_step1.txt",
          "active_task": "task_id",
          "task_count": 2,
          "has_progress": true
        },
        "events": [
          {"type": "TaskStarted", "task_id": "..."},
          {"type": "TaskProgress", "progress": 50}
        ]
      }
    }
  ]
}
```

## Field Definitions

### Top Level

- **name**: Human-readable test name
- **description**: What the scenario tests
- **config**: Test configuration
  - **terminal_size**: [width, height] for UI rendering
  - **enable_vector_cache**: Whether to simulate vector cache
  - **mock_embedding_speed**: Files per second for embedding simulation
- **initial_state**: Starting state of the system
- **steps**: Array of test steps to execute

### Initial State

- **workspace_files**: Mock files to create in test workspace
  - **path**: Relative path from workspace root
  - **content**: File content
- **existing_tasks**: Tasks that already exist (for testing persistence)
- **vector_cache_state**: State of vector cache
  - `"empty"`: No cache exists
  - `"partial"`: Some files cached
  - `"complete"`: All files cached

### Step Actions

#### user_input
Simulates user typing and submitting input
```json
{
  "type": "user_input",
  "data": {
    "text": "Fix the authentication bug",
    "submit": true
  }
}
```

#### agent_response
Mocked agent response with tool calls
```json
{
  "type": "agent_response",
  "data": {
    "text": "I'll analyze the authentication module",
    "tool_calls": [
      {
        "tool": "read_file",
        "args": {"path": "src/auth.rs"}
      }
    ],
    "subtasks": [
      {
        "description": "Analyze authentication flow",
        "priority": "high"
      }
    ]
  }
}
```

#### tool_result
Mocked tool execution result
```json
{
  "type": "tool_result",
  "data": {
    "tool": "read_file",
    "success": true,
    "output": "File content here..."
  }
}
```

#### wait
Wait for a duration (for testing async operations)
```json
{
  "type": "wait",
  "data": {
    "duration_ms": 100
  }
}
```

#### background_event
Simulate background task events (embedding, etc.)
```json
{
  "type": "background_event",
  "data": {
    "event_type": "embedding_progress",
    "progress": 50,
    "total": 100
  }
}
```

### Expectations

#### tasks
Verify task spawning and completion
```json
{
  "spawned": [
    {
      "description": "Fix the authentication bug",
      "status": "running",
      "has_progress": false
    }
  ],
  "completed": ["task_id_1"],
  "no_unexpected": true
}
```

#### background_tasks
Verify background tasks like embedding cache
```json
{
  "embedding_cache": {
    "status": "running",
    "progress": 42,
    "total": 100,
    "message": "Embedding src/main.rs"
  }
}
```

#### ui_state
Verify UI rendering and state
```json
{
  "snapshot": "scenario_name_step1.txt",
  "active_task": "task_id",
  "task_count": 2,
  "has_progress": true,
  "focused_pane": "output",
  "visible_tasks": [
    {
      "description": "Building embedding index",
      "status": "running"
    }
  ]
}
```

#### events
Verify specific UI events were emitted
```json
[
  {
    "type": "TaskStarted",
    "task_id": "...",
    "description": "Fix bug"
  },
  {
    "type": "TaskProgress",
    "progress": {
      "current": 50,
      "total": 100
    }
  },
  {
    "type": "ToolCallStarted",
    "tool": "read_file"
  }
]
```

## Example Scenarios

### 1. Simple User Input
```json
{
  "name": "User Input and Task Creation",
  "description": "User types input and submits, task is created",
  "config": {
    "terminal_size": [80, 30]
  },
  "initial_state": {
    "workspace_files": [],
    "vector_cache_state": "empty"
  },
  "steps": [
    {
      "step": 1,
      "action": {
        "type": "user_input",
        "data": {
          "text": "hello world",
          "submit": true
        }
      },
      "expectations": {
        "tasks": {
          "spawned": [
            {
              "description": "hello world",
              "status": "running"
            }
          ],
          "no_unexpected": true
        },
        "ui_state": {
          "snapshot": "user_input_step1.txt",
          "task_count": 1
        }
      }
    }
  ]
}
```

### 2. Vector Cache Initialization
```json
{
  "name": "Vector Cache Background Task",
  "description": "Vector cache starts immediately and shows progress",
  "config": {
    "terminal_size": [80, 30],
    "enable_vector_cache": true,
    "mock_embedding_speed": 10
  },
  "initial_state": {
    "workspace_files": [
      {"path": "src/main.rs", "content": "fn main() {}"},
      {"path": "src/lib.rs", "content": "pub fn test() {}"}
    ],
    "vector_cache_state": "empty"
  },
  "steps": [
    {
      "step": 1,
      "action": {
        "type": "wait",
        "data": {"duration_ms": 50}
      },
      "expectations": {
        "background_tasks": {
          "embedding_cache": {
            "status": "running",
            "progress": 0,
            "total": 2
          }
        },
        "tasks": {
          "spawned": [
            {
              "description": "Building embedding index",
              "status": "running",
              "has_progress": true
            }
          ]
        },
        "ui_state": {
          "snapshot": "vector_cache_step1.txt",
          "has_progress": true
        }
      }
    },
    {
      "step": 2,
      "action": {
        "type": "wait",
        "data": {"duration_ms": 100}
      },
      "expectations": {
        "background_tasks": {
          "embedding_cache": {
            "status": "running",
            "progress": 1,
            "total": 2
          }
        },
        "ui_state": {
          "snapshot": "vector_cache_step2.txt"
        }
      }
    }
  ]
}
```

### 3. Agent with Tool Calls
```json
{
  "name": "Agent Tool Execution",
  "description": "Agent calls tools and processes results",
  "config": {
    "terminal_size": [80, 30]
  },
  "initial_state": {
    "workspace_files": [
      {"path": "config.json", "content": "{\"debug\": true}"}
    ],
    "vector_cache_state": "complete"
  },
  "steps": [
    {
      "step": 1,
      "action": {
        "type": "user_input",
        "data": {
          "text": "Read config.json",
          "submit": true
        }
      },
      "expectations": {
        "tasks": {
          "spawned": [{"description": "Read config.json"}]
        }
      }
    },
    {
      "step": 2,
      "action": {
        "type": "agent_response",
        "data": {
          "text": "I'll read the config file",
          "tool_calls": [
            {
              "tool": "read_file",
              "args": {"path": "config.json"}
            }
          ]
        }
      },
      "expectations": {
        "events": [
          {
            "type": "ToolCallStarted",
            "tool": "read_file"
          }
        ],
        "ui_state": {
          "snapshot": "agent_tool_step2.txt"
        }
      }
    },
    {
      "step": 3,
      "action": {
        "type": "tool_result",
        "data": {
          "tool": "read_file",
          "success": true,
          "output": "{\"debug\": true}"
        }
      },
      "expectations": {
        "events": [
          {
            "type": "ToolCallCompleted",
            "tool": "read_file",
            "success": true
          }
        ]
      }
    }
  ]
}
```

## Usage

```rust
use merlin_routing::tests::scenario_runner::ScenarioRunner;

#[tokio::test]
async fn test_scenario() {
    let scenario = ScenarioRunner::load("scenario_name.json").unwrap();
    scenario.run().await.unwrap();
}
```

## Benefits

1. **Unified Format**: All E2E tests use the same JSON structure
2. **Declarative**: Describe what should happen, not how to test it
3. **Comprehensive**: Covers UI, tasks, events, background operations
4. **Maintainable**: Easy to add new scenarios without writing Rust code
5. **Visual**: Snapshots provide visual regression testing
6. **Flexible**: Can test any combination of features
