//! Integration tests for chunking validation.

#[cfg(test)]
mod tests {
    use merlin_context::embedding::chunking::{
        FileChunk, MAX_CHUNK_TOKENS, MIN_CHUNK_TOKENS, chunk_file, estimate_tokens,
    };
    use merlin_deps::tracing::info;
    use std::env::current_dir;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Walk the crates directory and collect all source files
    fn collect_crates_source_files() -> Vec<PathBuf> {
        use merlin_deps::ignore::WalkBuilder;

        let project_root: PathBuf = current_dir()
            .ok()
            .and_then(|path| path.ancestors().nth(2).map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));

        let crates_dir = project_root.join("crates");
        if !crates_dir.exists() {
            return Vec::default();
        }

        let mut files = Vec::default();

        let walker = WalkBuilder::new(&crates_dir)
            .max_depth(None)
            .hidden(true)
            .git_ignore(true)
            .build();

        for entry in walker.filter_map(Result::ok) {
            let path = entry.path();

            if entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
                && let Some(ext) = path.extension().and_then(|ext_str| ext_str.to_str())
            {
                match ext {
                    "rs" | "md" | "toml" | "txt" | "yaml" | "yml" | "json" => {
                        files.push(path.to_path_buf());
                    }
                    _ => {}
                }
            }
        }

        files
    }

    /// Report violations found during validation
    fn report_violations(violations: &[String], violation_type: &str) -> bool {
        if violations.is_empty() {
            return false;
        }

        info!(
            "\n❌ Found {} {} violations:",
            violations.len(),
            violation_type
        );
        for (index, violation) in violations.iter().enumerate().take(10) {
            info!("  {}. {}", index + 1, violation);
        }
        if violations.len() > 10 {
            info!("  ... and {} more", violations.len() - 10);
        }
        true
    }

    /// Validate a single file's chunks
    fn validate_file_chunks(
        file_path: &Path,
        content: &str,
        min_violations: &mut Vec<String>,
        max_violations: &mut Vec<String>,
        line_violations: &mut Vec<String>,
    ) -> (usize, bool) {
        let line_count = content.lines().count();
        let chunks = chunk_file(file_path, content);

        if chunks.is_empty() {
            return (0, false);
        }

        let is_single_chunk = chunks.len() == 1;
        let chunk_count = chunks.len();

        for chunk in chunks {
            let tokens = estimate_tokens(&chunk.content);

            validate_chunk_min_tokens(&chunk, file_path, is_single_chunk, tokens, min_violations);

            if tokens > MAX_CHUNK_TOKENS {
                max_violations.push(format!(
                    "ABOVE MAX: {}:{}-{} [{}] - {} tokens (max: {})",
                    file_path.display(),
                    chunk.start_line,
                    chunk.end_line,
                    chunk.identifier,
                    tokens,
                    MAX_CHUNK_TOKENS
                ));
            }

            validate_chunk_line_numbers(&chunk, file_path, line_count, line_violations);
        }

        (chunk_count, true)
    }

    #[test]
    fn test_chunking_validation() {
        let files = collect_crates_source_files();
        info!("Testing {} files for chunking compliance...", files.len());

        let mut min_violations = Vec::default();
        let mut max_violations = Vec::default();
        let mut line_violations = Vec::default();
        let mut total_chunks = 0;
        let mut files_tested = 0;

        for file_path in files {
            let Ok(content) = fs::read_to_string(&file_path) else {
                continue;
            };

            if content.trim().is_empty() {
                continue;
            }

            let (chunk_count, validated) = validate_file_chunks(
                &file_path,
                &content,
                &mut min_violations,
                &mut max_violations,
                &mut line_violations,
            );

            if validated {
                files_tested += 1;
                total_chunks += chunk_count;
            }
        }

        info!("Tested {files_tested} files, {total_chunks} total chunks in crates/ directory");

        let has_min_violations = report_violations(&min_violations, "MIN_CHUNK_TOKENS");
        let has_max_violations = report_violations(&max_violations, "MAX_CHUNK_TOKENS");
        let has_line_violations = report_violations(&line_violations, "line number");

        assert!(
            !has_min_violations && !has_max_violations && !has_line_violations,
            "❌ Chunking validation failed with {} total violations",
            min_violations.len() + max_violations.len() + line_violations.len()
        );

        info!("✅ All chunks pass validation (min/max tokens, line numbers)");
    }

    /// Validate a single chunk for minimum token compliance
    fn validate_chunk_min_tokens(
        chunk: &FileChunk,
        file_path: &Path,
        is_single_chunk: bool,
        tokens: usize,
        violations: &mut Vec<String>,
    ) {
        // Allow first chunk to be below minimum if it's the only chunk
        if is_single_chunk && tokens < MIN_CHUNK_TOKENS {
            // This is acceptable - single small file
            return;
        }

        // Check for zero-line chunks (CRITICAL - must never happen)
        if chunk.content.trim().is_empty() {
            violations.push(format!(
                "EMPTY CHUNK: {}:{}-{} [{}] - content is empty!",
                file_path.display(),
                chunk.start_line,
                chunk.end_line,
                chunk.identifier
            ));
        }

        // Check minimum token requirement (strict enforcement)
        if tokens < MIN_CHUNK_TOKENS {
            violations.push(format!(
                "BELOW MIN: {}:{}-{} [{}] - {} tokens (min: {})",
                file_path.display(),
                chunk.start_line,
                chunk.end_line,
                chunk.identifier,
                tokens,
                MIN_CHUNK_TOKENS
            ));
        }
    }

    /// Validate chunk line numbers
    fn validate_chunk_line_numbers(
        chunk: &FileChunk,
        file_path: &Path,
        line_count: usize,
        violations: &mut Vec<String>,
    ) {
        // Check start line is valid
        if chunk.start_line == 0 {
            violations.push(format!(
                "INVALID START: {}:{}-{} [{}] - start_line is 0 (should be 1-indexed)",
                file_path.display(),
                chunk.start_line,
                chunk.end_line,
                chunk.identifier
            ));
        }

        // Check end line is valid
        if chunk.end_line > line_count {
            violations.push(format!(
                "INVALID END: {}:{}-{} [{}] - end_line {} exceeds file length {}",
                file_path.display(),
                chunk.start_line,
                chunk.end_line,
                chunk.identifier,
                chunk.end_line,
                line_count
            ));
        }

        // Check start <= end
        if chunk.start_line > chunk.end_line {
            violations.push(format!(
                "INVALID RANGE: {}:{}-{} [{}] - start_line > end_line",
                file_path.display(),
                chunk.start_line,
                chunk.end_line,
                chunk.identifier
            ));
        }
    }
}
