//! File verification logic.

use super::fixture::FileVerify;
use super::verification_result::VerificationResult;
use std::fs;
use std::path::Path;

/// File verifier helper
pub struct FileVerifier;

impl FileVerifier {
    /// Verify file
    pub fn verify_file(
        result: &mut VerificationResult,
        workspace_root: &Path,
        verify: &FileVerify,
    ) {
        let file_path = workspace_root.join(&verify.path);

        // Check existence
        if !Self::verify_file_existence(result, verify, &file_path) {
            return;
        }

        // If file doesn't exist, skip content checks
        if !file_path.exists() {
            return;
        }

        // Read file content
        let content = match fs::read_to_string(&file_path) {
            Ok(file_content) => file_content,
            Err(err) => {
                result.add_failure(format!("Failed to read file {}: {err}", verify.path));
                return;
            }
        };

        Self::verify_file_content(result, verify, &content);
        Self::verify_file_size(result, verify, &content);
    }

    /// Verify file existence
    fn verify_file_existence(
        result: &mut VerificationResult,
        verify: &FileVerify,
        file_path: &Path,
    ) -> bool {
        if let Some(should_exist) = verify.exists {
            if file_path.exists() == should_exist {
                if should_exist {
                    result.add_success(format!("File {} exists", verify.path));
                } else {
                    result.add_success(format!("File {} does not exist", verify.path));
                }
            } else if should_exist {
                result.add_failure(format!("File {} does not exist", verify.path));
                return false;
            } else {
                result.add_failure(format!("File {} exists but should not", verify.path));
                return false;
            }
        }
        true
    }

    /// Verify file content patterns
    fn verify_file_content(result: &mut VerificationResult, verify: &FileVerify, content: &str) {
        // Check exact content
        if let Some(exact) = &verify.exact_content {
            if content == exact {
                result.add_success(format!("File {} has exact content", verify.path));
            } else {
                result.add_failure(format!(
                    "File {} content mismatch.\nExpected:\n{exact}\n\nActual:\n{content}",
                    verify.path
                ));
            }
        }

        // Check contains
        for pattern in &verify.contains {
            if content.contains(pattern) {
                result.add_success(format!("File {} contains '{pattern}'", verify.path));
            } else {
                result.add_failure(format!("File {} missing pattern '{pattern}'", verify.path));
            }
        }

        // Check not_contains
        for pattern in &verify.not_contains {
            if content.contains(pattern) {
                result.add_failure(format!(
                    "File {} contains forbidden pattern '{pattern}'",
                    verify.path
                ));
            } else {
                result.add_success(format!(
                    "File {} correctly does not contain '{pattern}'",
                    verify.path
                ));
            }
        }
    }

    /// Verify file size constraints
    fn verify_file_size(result: &mut VerificationResult, verify: &FileVerify, content: &str) {
        if let Some(min_size) = verify.size_gt {
            if content.len() > min_size {
                result.add_success(format!(
                    "File {} size {} > {min_size}",
                    verify.path,
                    content.len()
                ));
            } else {
                result.add_failure(format!(
                    "File {} size {} <= {min_size}",
                    verify.path,
                    content.len()
                ));
            }
        }

        if let Some(max_size) = verify.size_lt {
            if content.len() < max_size {
                result.add_success(format!(
                    "File {} size {} < {max_size}",
                    verify.path,
                    content.len()
                ));
            } else {
                result.add_failure(format!(
                    "File {} size {} >= {max_size}",
                    verify.path,
                    content.len()
                ));
            }
        }
    }
}
