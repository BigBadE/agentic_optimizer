//! Citation validation for agent responses.
//!
//! Validates that agent responses cite sources from the provided context,
//! ensuring traceability and accountability.

use merlin_core::{Severity, StageResult, ValidationError, ValidationStageType};
use merlin_deps::regex::Regex;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Instant;

/// Regex pattern for matching citations in format `<file:line>` or `<file:line1-line2>`
static CITATION_REGEX: LazyLock<Regex> =
    LazyLock::new(
        || match Regex::new(r"([a-zA-Z0-9_/\\.-]+\.[a-zA-Z0-9]+):(\d+)(?:-(\d+))?") {
            Ok(regex) => regex,
            Err(err) => panic!("Citation regex is invalid: {err}"),
        },
    );

/// Citation format: `file/path.rs:42-50` or `file/path.rs:42`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// File path referenced
    pub file_path: PathBuf,
    /// Start line number (if specified)
    pub start_line: Option<usize>,
    /// End line number (if specified, for ranges)
    pub end_line: Option<usize>,
}

impl Citation {
    /// Parse a citation from text
    ///
    /// Formats supported:
    /// - `path/to/file.rs:42`
    /// - `path/to/file.rs:42-50`
    /// - `crates/merlin-core/src/lib.rs:10-20`
    pub fn parse(text: &str) -> Option<Self> {
        // Pattern: file_path:line or file_path:line1-line2
        if let Some(captures) = CITATION_REGEX.captures(text) {
            let file_path = PathBuf::from(captures.get(1)?.as_str());
            let start_line = captures.get(2)?.as_str().parse().ok();
            let end_line = captures
                .get(3)
                .and_then(|match_group| match_group.as_str().parse().ok());

            Some(Self {
                file_path,
                start_line,
                end_line,
            })
        } else {
            None
        }
    }

    /// Extract all citations from text
    pub fn extract_all(text: &str) -> Vec<Self> {
        CITATION_REGEX
            .captures_iter(text)
            .filter_map(|cap| {
                let file_path = PathBuf::from(cap.get(1)?.as_str());
                let start_line = cap.get(2)?.as_str().parse().ok();
                let end_line = cap
                    .get(3)
                    .and_then(|match_group| match_group.as_str().parse().ok());

                Some(Self {
                    file_path,
                    start_line,
                    end_line,
                })
            })
            .collect()
    }
}

/// Citation validator
pub struct CitationValidator {
    /// Available context files
    context_files: HashSet<PathBuf>,
    /// Minimum citations required
    min_citations: usize,
    /// Whether to require citations (false = warning only)
    enforce: bool,
}

impl CitationValidator {
    /// Create a new citation validator
    pub fn new(context_files: Vec<PathBuf>) -> Self {
        Self {
            context_files: context_files.into_iter().collect(),
            min_citations: 1,
            enforce: false, // Start with warnings
        }
    }

    /// Set minimum required citations
    #[must_use]
    pub fn with_min_citations(mut self, min: usize) -> Self {
        self.min_citations = min;
        self
    }

    /// Enable enforcement (errors instead of warnings)
    #[must_use]
    pub fn with_enforcement(mut self, enforce: bool) -> Self {
        self.enforce = enforce;
        self
    }

    /// Validate a response for citations
    pub fn validate(&self, response: &str) -> StageResult {
        let start = Instant::now();

        let citations = Citation::extract_all(response);
        let mut errors = Vec::new();

        // Check citation count
        if citations.len() < self.min_citations {
            let message = format!(
                "Response contains {} citations but {} required",
                citations.len(),
                self.min_citations
            );

            if self.enforce {
                errors.push(ValidationError {
                    stage: ValidationStageType::Lint,
                    message,
                    severity: Severity::Error,
                });
            }
        }

        // Verify citations reference valid context files
        let mut invalid_citations = 0;
        for citation in &citations {
            if !self.is_valid_citation(citation) {
                invalid_citations += 1;

                let message = format!(
                    "Citation references unknown file: {}",
                    citation.file_path.display()
                );

                if self.enforce {
                    errors.push(ValidationError {
                        stage: ValidationStageType::Lint,
                        message,
                        severity: Severity::Warning,
                    });
                }
            }
        }

        // Calculate score
        let citation_score = if self.min_citations > 0 {
            (citations.len().min(self.min_citations) as f64 / self.min_citations as f64) * 0.5
        } else {
            0.5
        };

        let validity_score = if citations.is_empty() {
            0.5
        } else {
            let valid_count = citations.len() - invalid_citations;
            (valid_count as f64 / citations.len() as f64) * 0.5
        };

        let score = citation_score + validity_score;

        let details = format!(
            "Found {} citations ({} valid, {} invalid)",
            citations.len(),
            citations.len() - invalid_citations,
            invalid_citations
        );

        StageResult {
            stage: ValidationStageType::Lint,
            passed: errors.is_empty(),
            duration_ms: start.elapsed().as_millis() as u64,
            details,
            score,
        }
    }

    /// Check if a citation references a valid context file
    fn is_valid_citation(&self, citation: &Citation) -> bool {
        // Check exact match
        if self.context_files.contains(&citation.file_path) {
            return true;
        }

        // Check if citation is a suffix of any context file
        // (handles relative vs absolute paths)
        self.context_files
            .iter()
            .any(|ctx_file| ctx_file.ends_with(&citation.file_path))
    }

    /// Get citation statistics from response
    pub fn get_statistics(&self, response: &str) -> CitationStatistics {
        let citations = Citation::extract_all(response);
        let valid_citations = citations
            .iter()
            .filter(|citation| self.is_valid_citation(citation))
            .count();

        let unique_files: HashSet<_> = citations
            .iter()
            .map(|citation| &citation.file_path)
            .collect();

        CitationStatistics {
            total_citations: citations.len(),
            valid_citations,
            invalid_citations: citations.len() - valid_citations,
            unique_files_cited: unique_files.len(),
            has_line_numbers: citations
                .iter()
                .any(|citation| citation.start_line.is_some()),
        }
    }
}

/// Citation statistics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CitationStatistics {
    /// Total number of citations found
    pub total_citations: usize,
    /// Number of valid citations (reference context files)
    pub valid_citations: usize,
    /// Number of invalid citations
    pub invalid_citations: usize,
    /// Number of unique files cited
    pub unique_files_cited: usize,
    /// Whether any citations include line numbers
    pub has_line_numbers: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests parsing a simple citation with file path and line number.
    ///
    /// # Panics
    /// Panics if citation parsing fails or parsed values don't match expected values.
    #[test]
    fn test_parse_citation_simple() {
        let citation = Citation::parse("src/lib.rs:42");
        assert!(citation.is_some(), "Citation should parse successfully");

        if let Some(parsed) = citation {
            assert_eq!(parsed.file_path, PathBuf::from("src/lib.rs"));
            assert_eq!(parsed.start_line, Some(42));
            assert_eq!(parsed.end_line, None);
        }
    }

    /// Tests parsing a citation with a line range.
    ///
    /// # Panics
    /// Panics if citation parsing fails or parsed values don't match expected values.
    #[test]
    fn test_parse_citation_range() {
        let citation = Citation::parse("crates/merlin-core/src/lib.rs:10-20");
        assert!(
            citation.is_some(),
            "Citation with range should parse successfully"
        );

        if let Some(parsed) = citation {
            assert_eq!(
                parsed.file_path,
                PathBuf::from("crates/merlin-core/src/lib.rs")
            );
            assert_eq!(parsed.start_line, Some(10));
            assert_eq!(parsed.end_line, Some(20));
        }
    }

    /// Tests extracting all citations from text.
    ///
    /// # Panics
    /// Panics if citation extraction fails or extracted citations don't match expected values.
    #[test]
    fn test_extract_all_citations() {
        let text = "See src/lib.rs:10 and src/main.rs:20-30 for details.";
        let citations = Citation::extract_all(text);

        assert_eq!(citations.len(), 2);
        assert_eq!(citations[0].file_path, PathBuf::from("src/lib.rs"));
        assert_eq!(citations[0].start_line, Some(10));
        assert_eq!(citations[1].file_path, PathBuf::from("src/main.rs"));
        assert_eq!(citations[1].start_line, Some(20));
        assert_eq!(citations[1].end_line, Some(30));
    }

    /// Tests that citation validator produces warnings without enforcement.
    ///
    /// # Panics
    /// Panics if validation results don't match expected behavior.
    #[test]
    fn test_citation_validator_warnings() {
        let context_files = vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")];

        let validator = CitationValidator::new(context_files).with_min_citations(1);

        // Response with no citations
        let result_no_citations = validator.validate("This is a response without citations.");
        assert!(result_no_citations.passed); // Warnings don't fail
        assert!(result_no_citations.score < 1.0);

        // Response with valid citation
        let result_with_citation = validator.validate("See src/lib.rs:10 for implementation.");
        assert!(result_with_citation.passed);
        assert!(result_with_citation.score > 0.5);
    }

    /// Tests that citation validator fails with enforcement enabled.
    ///
    /// # Panics
    /// Panics if validation results don't match expected behavior.
    #[test]
    fn test_citation_validator_enforcement() {
        let context_files = vec![PathBuf::from("src/lib.rs")];

        let validator = CitationValidator::new(context_files)
            .with_min_citations(1)
            .with_enforcement(true);

        // Response with no citations should fail with enforcement
        let result = validator.validate("This is a response without citations.");
        assert!(!result.passed);
    }

    /// Tests citation statistics collection from response text.
    ///
    /// # Panics
    /// Panics if statistics don't match expected values.
    #[test]
    fn test_citation_statistics() {
        let context_files = vec![PathBuf::from("src/lib.rs"), PathBuf::from("src/main.rs")];

        let validator = CitationValidator::new(context_files);

        let response = "See src/lib.rs:10 and src/main.rs:20-30 and unknown.rs:5.";
        let stats = validator.get_statistics(response);

        assert_eq!(stats.total_citations, 3);
        assert_eq!(stats.valid_citations, 2);
        assert_eq!(stats.invalid_citations, 1);
        assert_eq!(stats.unique_files_cited, 3);
        assert!(stats.has_line_numbers);
    }
}
