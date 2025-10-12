//! Unit tests organized by component

// Make common module available to all submodules
#[path = "common/mod.rs"]
pub mod common;

// Input tests
#[path = "unit/input/input_manager_tests.rs"]
mod input_manager_tests;
#[path = "unit/input/input_manager_comprehensive_tests.rs"]
mod input_manager_comprehensive_tests;

// Output tests
#[path = "unit/output/output_tree_tests.rs"]
mod output_tree_tests;

// Task tests
#[path = "unit/tasks/task_manager_tests.rs"]
mod task_manager_tests;

// UI tests
#[path = "unit/ui/tui_edge_cases_tests.rs"]
mod tui_edge_cases_tests;
#[path = "unit/ui/tui_rendering_tests.rs"]
mod tui_rendering_tests;
#[path = "unit/ui/ui_event_tests.rs"]
mod ui_event_tests;
#[path = "unit/ui/ui_events_tests.rs"]
mod ui_events_tests;
#[path = "unit/ui/test_autowrap_fix.rs"]
mod test_autowrap_fix;

// Validation tests
#[path = "unit/validation/validator_tests.rs"]
mod validator_tests;
