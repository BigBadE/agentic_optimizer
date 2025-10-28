pub fn calculate_hash(input: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

pub fn validate_email(email: &str) -> bool {
    email.contains('@') && email.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email() {
        assert!(validate_email("test@example.com"));
        assert!(!validate_email("invalid"));
    }

    #[test]
    fn test_calculate_hash() {
        let hash1 = calculate_hash("test");
        let hash2 = calculate_hash("test");
        assert_eq!(hash1, hash2);
    }
}
