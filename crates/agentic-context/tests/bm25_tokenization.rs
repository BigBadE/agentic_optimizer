use agentic_context::embedding::BM25Index;

#[test]
fn test_special_token_preservation() {
    let mut index = BM25Index::new();
    
    // Add document with special tokens
    index.add_document(
        "test.rs".into(),
        "UserService::find_by_email implements the --verbose flag"
    );
    index.finalize();
    
    // Search for special tokens should work
    let results = index.search("UserService::find_by_email", 5);
    assert!(!results.is_empty(), "Should find document with :: token");
    assert_eq!(results[0].0.to_str().unwrap(), "test.rs");
    
    let results = index.search("--verbose", 5);
    assert!(!results.is_empty(), "Should find document with -- flag");
    assert_eq!(results[0].0.to_str().unwrap(), "test.rs");
}

#[test]
fn test_tokenization_debug() {
    // This test helps debug what tokens are actually generated
    let mut index = BM25Index::new();
    
    index.add_document(
        "file1.rs".into(),
        "UserService::find_by_email"
    );
    index.add_document(
        "file2.rs".into(),
        "UserService find by email"
    );
    index.finalize();
    
    // Exact match with :: should rank higher than without
    let results = index.search("UserService::find_by_email", 5);
    assert!(!results.is_empty(), "Should find results");
    
    // file1.rs should rank higher because it has the exact :: token
    if results.len() >= 2 {
        assert_eq!(results[0].0.to_str().unwrap(), "file1.rs", 
            "File with :: should rank higher. Scores: {:?}", results);
    }
}

#[test]
fn test_bigram_generation() {
    let mut index = BM25Index::new();
    
    index.add_document(
        "test.rs".into(),
        "authentication service implementation"
    );
    index.finalize();
    
    // Bigrams should help match phrases
    let results = index.search("authentication service", 5);
    assert!(!results.is_empty(), "Should find document with bigram");
}

#[test]
fn test_mixed_special_and_regular_tokens() {
    let mut index = BM25Index::new();
    
    index.add_document(
        "cli.rs".into(),
        "The --prompt flag accepts user::input::data"
    );
    index.finalize();
    
    // Both special tokens should be searchable
    let results = index.search("--prompt", 5);
    assert!(!results.is_empty(), "Should find --prompt flag");
    
    // Full path should match
    let results = index.search("user::input::data", 5);
    assert!(!results.is_empty(), "Should find user::input::data path");
    
    // Partial path should also match (because cleaned version is indexed)
    let results = index.search("user input", 5);
    assert!(!results.is_empty(), "Should find with cleaned tokens");
}
