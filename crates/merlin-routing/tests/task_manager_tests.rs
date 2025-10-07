/// Comprehensive tests for TaskManager functionality
mod common;

use common::*;
use merlin_routing::TaskId;
use merlin_routing::user_interface::task_manager::{TaskManager, TaskStatus};
use std::time::{Duration, Instant};

#[test]
fn test_add_task() {
    let mut manager = TaskManager::new();
    let task_id = TaskId::new();
    let task = create_test_task("Test task");

    manager.add_task(task_id, task);

    assert_eq!(manager.task_order().len(), 1);
    assert!(manager.get_task(task_id).is_some());
}

#[test]
fn test_remove_task() {
    let mut manager = TaskManager::new();
    let task_id = TaskId::new();

    manager.add_task(task_id, create_test_task("Test"));
    let removed = manager.remove_task(task_id);

    assert_eq!(removed.len(), 1);
    assert!(manager.is_empty());
}

#[test]
fn test_remove_task_with_children() {
    let mut manager = TaskManager::new();

    let parent_id = TaskId::new();
    let child1_id = TaskId::new();
    let child2_id = TaskId::new();

    manager.add_task(parent_id, create_test_task("Parent"));
    manager.add_task(child1_id, create_child_task("Child 1", parent_id));
    manager.add_task(child2_id, create_child_task("Child 2", parent_id));

    let removed = manager.remove_task(parent_id);

    // Should remove parent + 2 children
    assert_eq!(removed.len(), 3);
    assert!(manager.is_empty());
}

#[test]
fn test_collapse_expand() {
    let mut manager = TaskManager::new();

    let parent_id = TaskId::new();
    let child_id = TaskId::new();

    manager.add_task(parent_id, create_test_task("Parent"));
    manager.add_task(child_id, create_child_task("Child", parent_id));

    // Initially both visible
    assert_eq!(manager.get_visible_tasks().len(), 2);

    // Collapse parent
    manager.collapse_task(parent_id);
    assert!(manager.is_collapsed(parent_id));
    assert_eq!(manager.get_visible_tasks().len(), 1);

    // Expand parent
    manager.expand_task(parent_id);
    assert!(!manager.is_collapsed(parent_id));
    assert_eq!(manager.get_visible_tasks().len(), 2);
}

#[test]
fn test_toggle_collapse() {
    let mut manager = TaskManager::new();
    let task_id = TaskId::new();

    manager.add_task(task_id, create_test_task("Test"));

    assert!(!manager.is_collapsed(task_id));

    manager.toggle_collapse(task_id);
    assert!(manager.is_collapsed(task_id));

    manager.toggle_collapse(task_id);
    assert!(!manager.is_collapsed(task_id));
}

#[test]
fn test_has_children() {
    let mut manager = TaskManager::new();

    let parent_id = TaskId::new();
    let child_id = TaskId::new();

    manager.add_task(parent_id, create_test_task("Parent"));
    assert!(!manager.has_children(parent_id));

    manager.add_task(child_id, create_child_task("Child", parent_id));
    assert!(manager.has_children(parent_id));
}

#[test]
fn test_task_order_preserved_after_rebuild() {
    let mut manager = TaskManager::new();
    let now = Instant::now();

    // Add older task
    let task1_id = TaskId::new();
    let task1 = create_test_task_with_time("First", now);
    manager.insert_task_for_load(task1_id, task1);

    // Add newer task
    let task2_id = TaskId::new();
    let task2 = create_test_task_with_time("Second", now + Duration::from_secs(1));
    manager.insert_task_for_load(task2_id, task2);

    // Rebuild order
    manager.rebuild_order();

    // Newer tasks should appear first (reverse chronological)
    assert_eq!(manager.task_order()[0], task2_id);
    assert_eq!(manager.task_order()[1], task1_id);
}

#[test]
fn test_is_descendant_of() {
    let mut manager = TaskManager::new();

    let grandparent_id = TaskId::new();
    let parent_id = TaskId::new();
    let child_id = TaskId::new();

    manager.add_task(grandparent_id, create_test_task("Grandparent"));
    manager.add_task(parent_id, create_child_task("Parent", grandparent_id));
    manager.add_task(child_id, create_child_task("Child", parent_id));

    // Child is descendant of parent and grandparent
    assert!(manager.is_descendant_of(child_id, parent_id));
    assert!(manager.is_descendant_of(child_id, grandparent_id));

    // Parent is descendant of grandparent but not child
    assert!(manager.is_descendant_of(parent_id, grandparent_id));
    assert!(!manager.is_descendant_of(parent_id, child_id));

    // Grandparent is not descendant of anyone
    assert!(!manager.is_descendant_of(grandparent_id, parent_id));
    assert!(!manager.is_descendant_of(grandparent_id, child_id));
}

#[test]
fn test_get_visible_tasks_with_nested_collapse() {
    let mut manager = TaskManager::new();

    let root_id = TaskId::new();
    let child1_id = TaskId::new();
    let child2_id = TaskId::new();
    let grandchild_id = TaskId::new();

    manager.add_task(root_id, create_test_task("Root"));
    manager.add_task(child1_id, create_child_task("Child 1", root_id));
    manager.add_task(child2_id, create_child_task("Child 2", root_id));
    manager.add_task(grandchild_id, create_child_task("Grandchild", child1_id));

    // All visible initially
    assert_eq!(manager.get_visible_tasks().len(), 4);

    // Collapse child1
    manager.collapse_task(child1_id);
    // Root, Child1, Child2 visible (grandchild hidden)
    assert_eq!(manager.get_visible_tasks().len(), 3);

    // Collapse root
    manager.collapse_task(root_id);
    // Only root visible
    assert_eq!(manager.get_visible_tasks().len(), 1);
}

#[test]
fn test_iter_tasks() {
    let mut manager = TaskManager::new();

    let id1 = TaskId::new();
    let id2 = TaskId::new();

    manager.add_task(id1, create_test_task("Task 1"));
    manager.add_task(id2, create_test_task("Task 2"));

    let tasks: Vec<_> = manager.iter_tasks().collect();
    assert_eq!(tasks.len(), 2);
}

#[test]
fn test_is_empty() {
    let mut manager = TaskManager::new();
    assert!(manager.is_empty());

    let id = TaskId::new();
    manager.add_task(id, create_test_task("Test"));
    assert!(!manager.is_empty());

    manager.remove_task(id);
    assert!(manager.is_empty());
}

#[test]
fn test_get_task_mut() {
    let mut manager = TaskManager::new();
    let task_id = TaskId::new();

    manager.add_task(task_id, create_test_task("Test"));

    {
        let task = manager.get_task_mut(task_id).expect("Task should exist");
        task.status = TaskStatus::Completed;
    }

    let task = manager.get_task(task_id).expect("Task should exist");
    assert_eq!(task.status, TaskStatus::Completed);
}
