//! Tests for BM25 tokenization and special token handling.

use merlin_context::embedding::BM25Index;

/// Ensures special tokens like `::` and `--` are preserved and searchable.
#[test]
fn test_special_token_preservation() {
    let mut index = BM25Index::default();

    // Add document with special tokens
    index.add_document(
        "test.rs".into(),
        "UserService::find_by_email implements the --verbose flag",
    );
    index.finalize();

    // Search for special tokens should work
    let results = index.search("UserService::find_by_email", 5);
    assert!(!results.is_empty(), "Should find document with :: token");
    assert_eq!(results[0].0.to_string_lossy(), "test.rs");

    let verbose_results = index.search("--verbose", 5);
    assert!(
        !verbose_results.is_empty(),
        "Should find document with -- flag"
    );
    assert_eq!(verbose_results[0].0.to_string_lossy(), "test.rs");
}

/// Debug helper to ensure exact tokens rank higher.
#[test]
fn test_tokenization_debug() {
    // This test helps debug what tokens are actually generated
    let mut index = BM25Index::default();

    index.add_document("file1.rs".into(), "UserService::find_by_email");
    index.add_document("file2.rs".into(), "UserService find by email");
    index.finalize();

    // Exact match with :: should rank higher than without
    let results = index.search("UserService::find_by_email", 5);
    assert!(!results.is_empty(), "Should find results");

    // file1.rs should rank higher because it has the exact :: token
    if results.len() >= 2 {
        assert_eq!(
            results[0].0.to_string_lossy(),
            "file1.rs",
            "File with :: should rank higher. Scores: {results:?}"
        );
    }
}

/// Bigrams should improve phrase matching quality by ranking exact phrases higher.
#[test]
fn test_bigram_phrase_ranking() {
    let mut index = BM25Index::default();

    // Document with phrase together vs split
    index.add_document("file1.rs".into(), "authentication service implementation");
    index.add_document("file2.rs".into(), "authentication code and service layer");
    index.finalize();

    // Search for exact phrase - should rank file with adjacent words higher
    let results = index.search("authentication service", 5);
    assert!(!results.is_empty(), "Should find documents");

    // file1 has "authentication service" together, should rank higher than file2
    if results.len() >= 2 {
        assert_eq!(
            results[0].0.to_string_lossy(),
            "file1.rs",
            "Document with adjacent phrase should rank higher. Results: {results:?}"
        );
    }
}

/// Mixed special and regular tokens should be searchable together.
#[test]
fn test_mixed_special_and_regular_tokens() {
    let mut index = BM25Index::default();

    index.add_document(
        "cli.rs".into(),
        "The --prompt flag accepts user::input::data",
    );
    index.finalize();

    // Both special tokens should be searchable
    let results = index.search("--prompt", 5);
    assert!(!results.is_empty(), "Should find --prompt flag");

    // Full path should match
    let data_results = index.search("user::input::data", 5);
    assert!(
        !data_results.is_empty(),
        "Should find user::input::data path"
    );

    // Partial path should also match (because cleaned version is indexed)
    let input_results = index.search("user input", 5);
    assert!(!input_results.is_empty(), "Should find with cleaned tokens");
}
