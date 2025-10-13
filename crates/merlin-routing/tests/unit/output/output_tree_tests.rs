//! Comprehensive tests for output tree structure and navigation
#![cfg_attr(
    test,
    allow(
        dead_code,
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::missing_panics_doc,
        clippy::missing_errors_doc,
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::tests_outside_test_module,
        reason = "Test allows"
    )
)]

use merlin_routing::user_interface::output_tree::{OutputNode, OutputTree, StepType};
use serde_json::json;

#[test]
fn test_add_step() {
    let mut tree = OutputTree::default();
    tree.add_step(
        "step1".to_owned(),
        StepType::Thinking,
        "Analyzing problem".to_owned(),
    );

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 1);
}

#[test]
fn test_add_nested_steps() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "parent".to_owned(),
        StepType::Thinking,
        "Parent step".to_owned(),
    );
    tree.add_step(
        "child1".to_owned(),
        StepType::ToolCall,
        "Child 1".to_owned(),
    );
    tree.add_step("child2".to_owned(), StepType::Output, "Child 2".to_owned());

    tree.complete_step("child2");
    tree.complete_step("child1");
    tree.complete_step("parent");

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 3, "Should have parent and 2 children");
}

#[test]
fn test_complete_step() {
    let mut tree = OutputTree::default();

    tree.add_step("step1".to_owned(), StepType::Thinking, "Step 1".to_owned());
    tree.add_step("step2".to_owned(), StepType::Output, "Step 2".to_owned());

    tree.complete_step("step2");
    tree.complete_step("step1");

    // Steps should still be visible after completion
    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 2);
}

#[test]
fn test_analysis_step_auto_collapse() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "analysis".to_owned(),
        StepType::Thinking,
        "Analysis phase".to_owned(),
    );
    tree.add_step(
        "substep".to_owned(),
        StepType::Output,
        "Sub analysis".to_owned(),
    );
    tree.complete_step("substep");
    tree.complete_step("analysis");

    // Analysis step should be auto-collapsed
    let nodes = tree.flatten_visible_nodes();
    // Parent is visible, child is hidden due to collapse
    assert_eq!(nodes.len(), 1, "Analysis step should be collapsed");
}

#[test]
fn test_add_text() {
    let mut tree = OutputTree::default();

    tree.add_text("Plain text output".to_owned());

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 1);

    if let OutputNode::Text { content } = nodes[0].0.node {
        assert_eq!(content, "Plain text output");
    } else {
        panic!("Expected Text node");
    }
}

#[test]
fn test_add_text_under_step() {
    let mut tree = OutputTree::default();

    tree.add_step("parent".to_owned(), StepType::Thinking, "Parent".to_owned());
    tree.add_text("Child text".to_owned());
    tree.complete_step("parent");

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 2, "Should have parent step and text child");
}

#[test]
fn test_complete_tool_call() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "tool_step".to_owned(),
        StepType::ToolCall,
        "Read file".to_owned(),
    );

    let result = json!({
        "success": true,
        "content": "File contents here"
    });

    tree.complete_tool_call("Read", &result);

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 1);
}

#[test]
fn test_navigation_up() {
    let mut tree = OutputTree::default();

    tree.add_step("step1".to_owned(), StepType::Thinking, "Step 1".to_owned());
    tree.complete_step("step1");
    tree.add_step("step2".to_owned(), StepType::Thinking, "Step 2".to_owned());
    tree.complete_step("step2");
    tree.add_step("step3".to_owned(), StepType::Thinking, "Step 3".to_owned());
    tree.complete_step("step3");

    // Start at index 0
    assert_eq!(tree.selected_index(), 0);

    // Try moving up (should stay at 0)
    tree.move_up();
    assert_eq!(tree.selected_index(), 0);

    // Move down twice
    tree.move_down();
    tree.move_down();
    assert_eq!(tree.selected_index(), 2);

    // Move up once
    tree.move_up();
    assert_eq!(tree.selected_index(), 1);
}

#[test]
fn test_navigation_down() {
    let mut tree = OutputTree::default();

    tree.add_step("step1".to_owned(), StepType::Thinking, "Step 1".to_owned());
    tree.complete_step("step1");
    tree.add_step("step2".to_owned(), StepType::Thinking, "Step 2".to_owned());
    tree.complete_step("step2");

    tree.move_down();
    assert_eq!(tree.selected_index(), 1);

    // Try moving down past end (should stay at last)
    tree.move_down();
    assert_eq!(tree.selected_index(), 1);
}

#[test]
fn test_move_to_start() {
    let mut tree = OutputTree::default();

    tree.add_step("step1".to_owned(), StepType::Thinking, "Step 1".to_owned());
    tree.complete_step("step1");
    tree.add_step("step2".to_owned(), StepType::Thinking, "Step 2".to_owned());
    tree.complete_step("step2");
    tree.add_step("step3".to_owned(), StepType::Thinking, "Step 3".to_owned());
    tree.complete_step("step3");

    tree.move_down();
    tree.move_down();
    assert_eq!(tree.selected_index(), 2);

    tree.move_to_start();
    assert_eq!(tree.selected_index(), 0);
}

#[test]
fn test_move_to_end() {
    let mut tree = OutputTree::default();

    tree.add_step("step1".to_owned(), StepType::Thinking, "Step 1".to_owned());
    tree.complete_step("step1");
    tree.add_step("step2".to_owned(), StepType::Thinking, "Step 2".to_owned());
    tree.complete_step("step2");
    tree.add_step("step3".to_owned(), StepType::Thinking, "Step 3".to_owned());
    tree.complete_step("step3");

    tree.move_to_end();
    assert_eq!(tree.selected_index(), 2);
}

#[test]
fn test_page_navigation() {
    let mut tree = OutputTree::default();

    for idx in 0..20 {
        tree.add_step(
            format!("step{idx}"),
            StepType::Thinking,
            format!("Step {idx}"),
        );
        tree.complete_step(&format!("step{idx}"));
    }

    tree.page_down(5);
    assert_eq!(tree.selected_index(), 5);

    tree.page_down(5);
    assert_eq!(tree.selected_index(), 10);

    tree.page_up(3);
    assert_eq!(tree.selected_index(), 7);
}

#[test]
fn test_collapse_expand() {
    let mut tree = OutputTree::default();

    tree.add_step("parent".to_owned(), StepType::Thinking, "Parent".to_owned());
    tree.add_step("child1".to_owned(), StepType::Output, "Child 1".to_owned());
    tree.complete_step("child1");
    tree.add_step("child2".to_owned(), StepType::Output, "Child 2".to_owned());
    tree.complete_step("child2");
    tree.complete_step("parent");

    // Initially expanded - should see all 3 nodes
    let nodes_expanded = tree.flatten_visible_nodes();
    assert_eq!(nodes_expanded.len(), 3);

    // Collapse parent (which is at index 0)
    tree.collapse_selected();

    // Now should only see parent
    let nodes_collapsed = tree.flatten_visible_nodes();
    assert_eq!(nodes_collapsed.len(), 1);

    // Expand again
    tree.expand_selected();

    let nodes_reexpanded = tree.flatten_visible_nodes();
    assert_eq!(nodes_reexpanded.len(), 3);
}

#[test]
fn test_toggle_collapse() {
    let mut tree = OutputTree::default();

    tree.add_step("parent".to_owned(), StepType::Thinking, "Parent".to_owned());
    tree.add_step("child".to_owned(), StepType::Output, "Child".to_owned());
    tree.complete_step("child");
    tree.complete_step("parent");

    // Initially expanded
    let nodes_expanded = tree.flatten_visible_nodes();
    assert_eq!(nodes_expanded.len(), 2);

    // Toggle to collapse
    tree.toggle_selected();
    let nodes_collapsed = tree.flatten_visible_nodes();
    assert_eq!(nodes_collapsed.len(), 1);

    // Toggle to expand
    tree.toggle_selected();
    let nodes_reexpanded = tree.flatten_visible_nodes();
    assert_eq!(nodes_reexpanded.len(), 2);
}

#[test]
fn test_deep_nesting() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "level1".to_owned(),
        StepType::Thinking,
        "Level 1".to_owned(),
    );
    tree.add_step(
        "level2".to_owned(),
        StepType::Thinking,
        "Level 2".to_owned(),
    );
    tree.add_step(
        "level3".to_owned(),
        StepType::Thinking,
        "Level 3".to_owned(),
    );
    tree.add_step("level4".to_owned(), StepType::Output, "Level 4".to_owned());
    tree.complete_step("level4");
    tree.complete_step("level3");
    tree.complete_step("level2");
    tree.complete_step("level1");

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 4, "Should have all 4 levels visible");

    // Check depths
    assert_eq!(nodes[0].1, 0, "Level 1 should be depth 0");
    assert_eq!(nodes[1].1, 1, "Level 2 should be depth 1");
    assert_eq!(nodes[2].1, 2, "Level 3 should be depth 2");
    assert_eq!(nodes[3].1, 3, "Level 4 should be depth 3");
}

#[test]
fn test_to_text() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "step1".to_owned(),
        StepType::Thinking,
        "First step".to_owned(),
    );
    tree.add_text("Some output".to_owned());
    tree.complete_step("step1");

    let text = tree.to_text();
    assert!(text.contains("First step"));
    assert!(text.contains("Some output"));
}

#[test]
fn test_empty_tree() {
    let tree = OutputTree::default();

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 0);

    let text = tree.to_text();
    assert!(text.is_empty() || text.trim().is_empty());
}

#[test]
fn test_step_type_parsing() {
    let thinking = StepType::from_string("Thinking");
    assert_eq!(thinking, StepType::Thinking);

    let tool_call = StepType::from_string("ToolCall");
    assert_eq!(tool_call, StepType::ToolCall);

    let subtask = StepType::from_string("Subtask");
    assert_eq!(subtask, StepType::Subtask);

    let output = StepType::from_string("Unknown");
    assert_eq!(output, StepType::Output);
}

#[test]
fn test_node_icons() {
    let mut tree = OutputTree::default();

    tree.add_step(
        "thinking".to_owned(),
        StepType::Thinking,
        "Think".to_owned(),
    );
    tree.complete_step("thinking");
    tree.add_step("tool".to_owned(), StepType::ToolCall, "Tool".to_owned());
    tree.complete_step("tool");
    tree.add_step("output".to_owned(), StepType::Output, "Out".to_owned());
    tree.complete_step("output");
    tree.add_step("subtask".to_owned(), StepType::Subtask, "Sub".to_owned());
    tree.complete_step("subtask");

    let nodes = tree.flatten_visible_nodes();

    // Each node should have an appropriate icon
    for (node_ref, _depth) in &nodes {
        let icon = node_ref.node.get_icon(false);
        assert!(!icon.is_empty(), "Icon should not be empty");
    }
}

#[test]
fn test_node_content() {
    let step = OutputNode::Step {
        id: "test".to_owned(),
        step_type: StepType::Thinking,
        content: "Test content".to_owned(),
        children: vec![],
    };

    let content = step.get_content();
    assert_eq!(content, "Test content");

    let text = OutputNode::Text {
        content: "Plain text".to_owned(),
    };

    let text_content = text.get_content();
    assert_eq!(text_content, "Plain text");
}

#[test]
fn test_mixed_step_types() {
    let mut tree = OutputTree::default();

    tree.add_step("parent".to_owned(), StepType::Thinking, "Parent".to_owned());
    tree.add_step(
        "tool_child".to_owned(),
        StepType::ToolCall,
        "Tool".to_owned(),
    );
    tree.complete_step("tool_child");
    tree.add_step(
        "output_child".to_owned(),
        StepType::Output,
        "Output".to_owned(),
    );
    tree.complete_step("output_child");
    tree.add_step(
        "subtask_child".to_owned(),
        StepType::Subtask,
        "Subtask".to_owned(),
    );
    tree.complete_step("subtask_child");
    tree.complete_step("parent");

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 4, "Parent + 3 children");
}

#[test]
fn test_sibling_steps() {
    let mut tree = OutputTree::default();

    tree.add_step("parent".to_owned(), StepType::Thinking, "Parent".to_owned());
    tree.add_step("child1".to_owned(), StepType::Output, "Child 1".to_owned());
    tree.complete_step("child1");
    tree.complete_step("parent");

    tree.add_step(
        "sibling".to_owned(),
        StepType::Thinking,
        "Sibling".to_owned(),
    );
    tree.complete_step("sibling");

    let nodes = tree.flatten_visible_nodes();
    assert_eq!(nodes.len(), 3, "Should have parent, child, and sibling");
}

#[test]
fn test_complex_tree_structure() {
    let mut tree = OutputTree::default();

    // Root 1 with children
    tree.add_step("root1".to_owned(), StepType::Thinking, "Root 1".to_owned());
    tree.add_step(
        "r1_child1".to_owned(),
        StepType::Output,
        "R1 Child 1".to_owned(),
    );
    tree.complete_step("r1_child1");
    tree.add_step(
        "r1_child2".to_owned(),
        StepType::Output,
        "R1 Child 2".to_owned(),
    );
    tree.add_step(
        "r1_c2_grandchild".to_owned(),
        StepType::Output,
        "Grandchild".to_owned(),
    );
    tree.complete_step("r1_c2_grandchild");
    tree.complete_step("r1_child2");
    tree.complete_step("root1");

    // Root 2
    tree.add_step("root2".to_owned(), StepType::Thinking, "Root 2".to_owned());
    tree.complete_step("root2");

    let nodes = tree.flatten_visible_nodes();
    // Root1, R1_Child1, R1_Child2, Grandchild, Root2 = 5 nodes
    assert_eq!(
        nodes.len(),
        5,
        "Should have 2 roots, 2 children, 1 grandchild"
    );
}

#[test]
fn test_navigation_with_collapsed_nodes() {
    let mut tree = OutputTree::default();

    tree.add_step("parent".to_owned(), StepType::Thinking, "Parent".to_owned());
    tree.add_step("child1".to_owned(), StepType::Output, "Child 1".to_owned());
    tree.complete_step("child1");
    tree.add_step("child2".to_owned(), StepType::Output, "Child 2".to_owned());
    tree.complete_step("child2");
    tree.complete_step("parent");

    tree.add_step(
        "sibling".to_owned(),
        StepType::Thinking,
        "Sibling".to_owned(),
    );
    tree.complete_step("sibling");

    // Collapse parent
    tree.collapse_selected();

    let visible = tree.flatten_visible_nodes();
    assert_eq!(visible.len(), 2, "Only parent and sibling visible");

    // Navigate down should skip hidden children
    tree.move_down();
    assert_eq!(tree.selected_index(), 1, "Should move to sibling");
}
