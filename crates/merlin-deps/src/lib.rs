//! Shared dependency re-exports for the Merlin workspace.
//!
//! This crate re-exports all external dependencies as a dynamically linked library,
//! improving incremental build times and reducing code duplication across test binaries.

// Core dependencies
pub use anyhow;
pub use async_trait;
pub use boa_engine;
pub use chrono;
pub use console;
pub use dirs;
pub use filetime;
pub use flate2;
pub use futures;
pub use glob;
pub use ignore;
pub use ollama_rs;
pub use petgraph;
pub use regex;
pub use reqwest;
pub use serde_json;
pub use tempfile;
pub use toml;
pub use tracing;
pub use tracing_subscriber;
pub use uuid;
pub use walkdir;

// TUI dependencies
pub use crossterm;
pub use ratatui;
pub use tui_textarea;
pub use unicode_width;

// SWC TypeScript transpiler
pub use swc_common;
pub use swc_ecma_ast;
pub use swc_ecma_codegen;
pub use swc_ecma_parser;
pub use swc_ecma_transforms_base;
pub use swc_ecma_transforms_typescript;
pub use swc_ecma_visit;
