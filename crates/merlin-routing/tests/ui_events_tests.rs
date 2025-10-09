//! Comprehensive tests for UI event types and structures
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::print_stderr,
    clippy::tests_outside_test_module,
    clippy::missing_panics_doc,
    clippy::min_ident_chars,
    clippy::needless_pass_by_value,
    reason = "Tests allow these"
)]

use merlin_routing::TaskId;
use merlin_routing::user_interface::{MessageLevel, TaskProgress, UiEvent};

#[test]
fn test_task_progress_creation() {
    let progress = TaskProgress {
        stage: "analyzing".to_string(),
        current: 5,
        total: Some(10),
        message: "Half done".to_string(),
    };

    assert_eq!(progress.stage, "analyzing");
    assert_eq!(progress.current, 5);
    assert_eq!(progress.total, Some(10));
    assert_eq!(progress.message, "Half done");
}

#[test]
fn test_task_progress_no_total() {
    let progress = TaskProgress {
        stage: "processing".to_string(),
        current: 42,
        total: None,
        message: "Processing items...".to_string(),
    };

    assert_eq!(progress.total, None);
    assert_eq!(progress.current, 42);
}

#[test]
fn test_message_level_types() {
    let info = MessageLevel::Info;
    let warning = MessageLevel::Warning;
    let error = MessageLevel::Error;

    // Just verify they exist and can be created
    assert!(matches!(info, MessageLevel::Info));
    assert!(matches!(warning, MessageLevel::Warning));
    assert!(matches!(error, MessageLevel::Error));
}

#[test]
fn test_task_started_event_structure() {
    let task_id = TaskId::default();
    let event = UiEvent::TaskStarted {
        task_id,
        description: "Test task".to_string(),
        parent_id: None,
    };

    match event {
        UiEvent::TaskStarted {
            task_id: id,
            description,
            parent_id,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(description, "Test task");
            assert_eq!(parent_id, None);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_task_started_with_parent() {
    let parent_id = TaskId::default();
    let child_id = TaskId::default();

    let event = UiEvent::TaskStarted {
        task_id: child_id,
        description: "Child task".to_string(),
        parent_id: Some(parent_id),
    };

    match event {
        UiEvent::TaskStarted { parent_id: pid, .. } => {
            assert_eq!(pid, Some(parent_id));
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_task_progress_event() {
    let task_id = TaskId::default();
    let progress = TaskProgress {
        stage: "executing".to_string(),
        current: 3,
        total: Some(5),
        message: "Working...".to_string(),
    };

    let event = UiEvent::TaskProgress { task_id, progress };

    match event {
        UiEvent::TaskProgress {
            task_id: id,
            progress: prog,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(prog.current, 3);
            assert_eq!(prog.total, Some(5));
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_task_output_event() {
    let task_id = TaskId::default();
    let event = UiEvent::TaskOutput {
        task_id,
        output: "Some output text".to_string(),
    };

    match event {
        UiEvent::TaskOutput {
            task_id: id,
            output,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(output, "Some output text");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_task_failed_event() {
    let task_id = TaskId::default();
    let event = UiEvent::TaskFailed {
        task_id,
        error: "Something went wrong".to_string(),
    };

    match event {
        UiEvent::TaskFailed { task_id: id, error } => {
            assert_eq!(id, task_id);
            assert_eq!(error, "Something went wrong");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_system_message_event() {
    let event = UiEvent::SystemMessage {
        level: MessageLevel::Info,
        message: "System message".to_string(),
    };

    match event {
        UiEvent::SystemMessage { level, message } => {
            assert!(matches!(level, MessageLevel::Info));
            assert_eq!(message, "System message");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_task_step_started_event() {
    let task_id = TaskId::default();
    let event = UiEvent::TaskStepStarted {
        task_id,
        step_id: "step-1".to_string(),
        step_type: "Thinking".to_string(),
        content: "Analyzing...".to_string(),
    };

    match event {
        UiEvent::TaskStepStarted {
            task_id: id,
            step_id,
            step_type,
            content,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(step_id, "step-1");
            assert_eq!(step_type, "Thinking");
            assert_eq!(content, "Analyzing...");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_task_step_completed_event() {
    let task_id = TaskId::default();
    let event = UiEvent::TaskStepCompleted {
        task_id,
        step_id: "step-1".to_string(),
    };

    match event {
        UiEvent::TaskStepCompleted {
            task_id: id,
            step_id,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(step_id, "step-1");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_tool_call_started_event() {
    let task_id = TaskId::default();
    let args = serde_json::json!({"path": "test.txt"});

    let event = UiEvent::ToolCallStarted {
        task_id,
        tool: "read_file".to_string(),
        args: args.clone(),
    };

    match event {
        UiEvent::ToolCallStarted {
            task_id: id,
            tool,
            args: tool_args,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(tool, "read_file");
            assert_eq!(tool_args, args);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_tool_call_completed_event() {
    let task_id = TaskId::default();
    let result = serde_json::json!({"success": true});

    let event = UiEvent::ToolCallCompleted {
        task_id,
        tool: "write_file".to_string(),
        result: result.clone(),
    };

    match event {
        UiEvent::ToolCallCompleted {
            task_id: id,
            tool,
            result: tool_result,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(tool, "write_file");
            assert_eq!(tool_result, result);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_thinking_update_event() {
    let task_id = TaskId::default();
    let event = UiEvent::ThinkingUpdate {
        task_id,
        content: "Considering options...".to_string(),
    };

    match event {
        UiEvent::ThinkingUpdate {
            task_id: id,
            content,
        } => {
            assert_eq!(id, task_id);
            assert_eq!(content, "Considering options...");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_subtask_spawned_event() {
    let parent_id = TaskId::default();
    let child_id = TaskId::default();

    let event = UiEvent::SubtaskSpawned {
        parent_id,
        child_id,
        description: "Subtask description".to_string(),
    };

    match event {
        UiEvent::SubtaskSpawned {
            parent_id: pid,
            child_id: cid,
            description,
        } => {
            assert_eq!(pid, parent_id);
            assert_eq!(cid, child_id);
            assert_eq!(description, "Subtask description");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_unicode_in_events() {
    let event = UiEvent::TaskOutput {
        task_id: TaskId::default(),
        output: "Hello 世界 🚀".to_string(),
    };

    match event {
        UiEvent::TaskOutput { output, .. } => {
            assert!(output.contains("世界"));
            assert!(output.contains("🚀"));
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_special_characters_in_events() {
    let special = "Quotes \"here\" and 'there' \n\t\r";
    let event = UiEvent::SystemMessage {
        level: MessageLevel::Warning,
        message: special.to_string(),
    };

    match event {
        UiEvent::SystemMessage { message, .. } => {
            assert_eq!(message, special);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_empty_strings_in_events() {
    let event = UiEvent::TaskOutput {
        task_id: TaskId::default(),
        output: String::new(),
    };

    match event {
        UiEvent::TaskOutput { output, .. } => {
            assert_eq!(output, "");
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_very_long_output() {
    let long_output = "x".repeat(10000);
    let expected_len = long_output.len();
    let event = UiEvent::TaskOutput {
        task_id: TaskId::default(),
        output: long_output,
    };

    match event {
        UiEvent::TaskOutput { output, .. } => {
            assert_eq!(output.len(), expected_len);
        }
        _ => panic!("Wrong event type"),
    }
}

#[test]
fn test_progress_clone() {
    let progress1 = TaskProgress {
        stage: "test".to_string(),
        current: 1,
        total: None,
        message: "msg".to_string(),
    };

    let progress2 = progress1.clone();

    assert_eq!(progress1.stage, progress2.stage);
    assert_eq!(progress1.current, progress2.current);
}
