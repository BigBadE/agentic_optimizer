//! Verification result types.

/// Verification result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Whether all verifications passed
    pub passed: bool,
    /// List of failures
    pub failures: Vec<String>,
    /// List of successes
    pub successes: Vec<String>,
}

impl VerificationResult {
    /// Create new verification result
    #[must_use]
    pub fn new() -> Self {
        Self {
            passed: true,
            failures: Vec::new(),
            successes: Vec::new(),
        }
    }

    /// Add success
    pub fn add_success(&mut self, message: String) {
        self.successes.push(message);
    }

    /// Add failure
    pub fn add_failure(&mut self, message: String) {
        self.passed = false;
        self.failures.push(message);
    }

    /// Merge another result
    pub fn merge(&mut self, other: Self) {
        if !other.passed {
            self.passed = false;
        }
        self.failures.extend(other.failures);
        self.successes.extend(other.successes);
    }
}

impl Default for VerificationResult {
    fn default() -> Self {
        Self::new()
    }
}
