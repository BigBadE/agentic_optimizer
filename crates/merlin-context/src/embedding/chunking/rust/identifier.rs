//! Rust identifier extraction from item declarations.

/// Extract identifier from Rust item declaration
pub fn extract_rust_identifier(line: &str) -> String {
    let line = line.trim();

    // Remove pub/async/const/unsafe modifiers
    let line = line
        .trim_start_matches("pub ")
        .trim_start_matches("async ")
        .trim_start_matches("const ")
        .trim_start_matches("unsafe ");

    // Extract based on keyword
    if let Some(rest) = line.strip_prefix("fn ") {
        return format!(
            "fn {}",
            rest.split(&['(', '<', ' '][..]).next().unwrap_or("unknown")
        );
    }
    if let Some(rest) = line.strip_prefix("struct ") {
        return format!(
            "struct {}",
            rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown")
        );
    }
    if let Some(rest) = line.strip_prefix("enum ") {
        return format!(
            "enum {}",
            rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown")
        );
    }
    if let Some(rest) = line.strip_prefix("trait ") {
        return format!(
            "trait {}",
            rest.split(&[' ', '<', '{'][..]).next().unwrap_or("unknown")
        );
    }
    if line.starts_with("impl ") || line.starts_with("impl<") {
        let impl_part = line.split('{').next().unwrap_or(line).trim();
        return format!(
            "impl {}",
            impl_part.strip_prefix("impl ").unwrap_or("").trim()
        );
    }
    if let Some(rest) = line.strip_prefix("mod ") {
        return format!(
            "mod {}",
            rest.split(&[' ', '{'][..]).next().unwrap_or("unknown")
        );
    }

    String::from("item")
}
