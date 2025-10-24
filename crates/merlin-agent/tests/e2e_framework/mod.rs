//! Comprehensive E2E testing framework for agent workflows.
//!
//! This module provides a complete testing infrastructure that:
//! - Uses real code paths (no mocking except provider responses)
//! - Verifies all execution details (tool calls, file operations, responses)
//! - Tracks execution flow for debugging
//! - Supports negative testing for error scenarios

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
        clippy::absolute_paths,
        clippy::min_ident_chars,
        clippy::use_self,
        clippy::uninlined_format_args,
        clippy::too_many_lines,
        reason = "E2E framework allows"
    )
)]

pub mod fixture;
pub mod mock_provider;
pub mod runner;
pub mod verifier;
