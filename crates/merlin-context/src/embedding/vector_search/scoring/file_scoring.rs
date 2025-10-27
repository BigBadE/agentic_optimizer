//! File-based scoring utilities.

use std::path::Path;

/// Calculate file type and location boost
pub fn calculate_file_boost(path: &Path) -> f32 {
    let path_str = path.to_str().unwrap_or("");
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

    // Heavy penalty for test files
    if path_str.contains("/tests/") || path_str.contains("\\tests\\") {
        return 0.1;
    }

    // Heavy penalty for benchmark files
    if path_str.contains("/benches/")
        || path_str.contains("\\benches\\")
        || path_str.contains("/benchmarks/")
        || path_str.contains("\\benchmarks\\")
    {
        return 0.1;
    }

    let mut type_boost = match ext {
        "rs" | "py" | "js" | "ts" | "jsx" | "tsx" | "java" | "c" | "cpp" | "h" | "hpp" | "go"
        | "rb" | "php" | "cs" | "swift" | "kt" | "scala" => 1.7,
        "toml" | "yaml" | "yml" | "json" | "xml" => 0.25, // Reduced by 50%
        "md" | "txt" => 0.05,                             // Reduced by 50%
        _ => 0.5,                                         // Reduced by 50%
    };

    // Boost module entry points
    if path_str.ends_with("/lib.rs") || path_str.ends_with("\\lib.rs") {
        type_boost *= 1.3; // Entry points are important
    } else if path_str.ends_with("/mod.rs") || path_str.ends_with("\\mod.rs") {
        type_boost *= 1.2; // Module definitions
    }

    let location_boost = if path_str.contains("/src/") || path_str.contains("\\src\\") {
        1.3
    } else if path_str.contains("/docs/")
        || path_str.contains("\\docs\\")
        || path_str.contains("/examples/")
        || path_str.contains("\\examples\\")
    {
        0.5
    } else {
        1.0
    };

    type_boost * location_boost
}
