use crate::common::*;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use merlin_routing::TaskId;
use merlin_routing::user_interface::output_tree::OutputTree;
use merlin_routing::user_interface::renderer::FocusedPane;
use merlin_routing::user_interface::task_manager::{TaskDisplay, TaskStatus};
use std::time::Instant;

#[test]
fn test_enter_toggles_expansion() {
    init_tracing();

    // Create app
    let (mut app, _) = create_test_app(80, 30).unwrap();

    // Add a conversation with children
    let parent_id = TaskId::default();
    let child_id = TaskId::default();

    let parent_task = TaskDisplay {
        description: "Parent conversation".to_string(),
        status: TaskStatus::Completed,
        start_time: Instant::now(),
        end_time: Some(Instant::now()),
        parent_id: None,
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    };

    let child_task = TaskDisplay {
        description: "Child message".to_string(),
        status: TaskStatus::Completed,
        start_time: Instant::now(),
        end_time: Some(Instant::now()),
        parent_id: Some(parent_id),
        progress: None,
        output_lines: vec![],
        output_tree: OutputTree::default(),
        steps: vec![],
    };

    app.task_manager_mut().add_task(parent_id, parent_task);
    app.task_manager_mut().add_task(child_id, child_task);

    // Focus Tasks pane and select the parent
    app.set_focused_pane(FocusedPane::Tasks);
    app.state_mut().active_task_id = Some(parent_id);

    // Verify not expanded initially
    assert!(!app.state().expanded_conversations.contains(&parent_id));

    // Press Enter
    let enter_event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.set_event_source(Box::new(TestEventSource::with_events(vec![enter_event])));
    app.tick().unwrap();

    // Verify expanded
    assert!(
        app.state().expanded_conversations.contains(&parent_id),
        "Expected parent conversation to be expanded after pressing Enter"
    );

    // Press Enter again
    let enter_event_2 = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    app.set_event_source(Box::new(TestEventSource::with_events(vec![enter_event_2])));
    app.tick().unwrap();

    // Verify collapsed
    assert!(
        !app.state().expanded_conversations.contains(&parent_id),
        "Expected parent conversation to be collapsed after pressing Enter again"
    );
}
