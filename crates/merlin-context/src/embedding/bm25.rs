//! BM25 keyword search implementation for file ranking.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

/// BM25 parameters
const TERM_SATURATION_K1: f32 = 1.5; // Term frequency saturation parameter
const LENGTH_NORM_B: f32 = 0.75; // Length normalization parameter

/// Document in the BM25 index
#[derive(Debug, Clone)]
struct Document {
    path: PathBuf,
    terms: HashMap<String, usize>, // term -> frequency
    length: usize,                 // total terms in document
}

/// BM25 search index
#[derive(Default)]
pub struct BM25Index {
    documents: Vec<Document>,
    avg_doc_length: f32,
    idf_cache: HashMap<String, f32>, // term -> IDF score
}

impl BM25Index {
    /// Common stop words that should not influence scoring
    #[allow(
        clippy::too_many_lines,
        reason = "Comprehensive stopword list required"
    )]
    fn stopwords() -> &'static HashSet<&'static str> {
        use std::sync::OnceLock;

        static STOPWORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
        STOPWORDS.get_or_init(|| {
            [
                "the",
                "and",
                "for",
                "with",
                "that",
                "from",
                "this",
                "have",
                "will",
                "into",
                "when",
                "where",
                "what",
                "your",
                "their",
                "about",
                "which",
                "there",
                "been",
                "while",
                "without",
                "should",
                "could",
                "would",
                "using",
                "used",
                "they",
                "them",
                "then",
                "than",
                "only",
                "also",
                "over",
                "under",
                "after",
                "before",
                "each",
                "every",
                "more",
                "most",
                "some",
                "such",
                "within",
                "between",
                "because",
                "again",
                "almost",
                "always",
                "never",
                "being",
                "having",
                "through",
                "across",
                "please",
                "however",
                "though",
                "whereas",
                "among",
                "amongst",
                "whose",
                "ourselves",
                "yourselves",
                "themselves",
                "itself",
                "hers",
                "his",
                "herself",
                "himself",
                "it",
                "its",
                "you",
                "we",
                "our",
                "ours",
                "can",
                "cannot",
                "can't",
                "cant",
            ]
            .into_iter()
            .collect()
        })
    }

    // Use Default instead of a no-arg constructor

    /// Add a document to the index
    pub fn add_document(&mut self, path: PathBuf, content: &str) {
        let terms = Self::tokenize(content);
        let term_freq = Self::count_terms(&terms);
        let length = terms.len();

        self.documents.push(Document {
            path,
            terms: term_freq,
            length,
        });

        // Invalidate IDF cache when adding documents
        self.idf_cache.clear();
    }

    /// Finalize the index (compute IDF scores)
    pub fn finalize(&mut self) {
        if self.documents.is_empty() {
            return;
        }

        // Compute average document length
        let total_length: usize = self.documents.iter().map(|document| document.length).sum();
        self.avg_doc_length = total_length as f32 / self.documents.len() as f32;

        // Compute IDF for all terms
        let mut doc_freq: HashMap<String, usize> = HashMap::default();
        for doc in &self.documents {
            for term in doc.terms.keys() {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let num_docs = self.documents.len() as f32;
        for (term, document_frequency) in doc_freq {
            let idf = ((num_docs - document_frequency as f32 + 0.5)
                / (document_frequency as f32 + 0.5))
                .ln_1p();
            self.idf_cache.insert(term, idf);
        }
    }

    /// Search for documents matching the query
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(PathBuf, f32)> {
        let query_terms = Self::tokenize(query);
        let mut scores: Vec<(PathBuf, f32)> = Vec::default();

        for doc in &self.documents {
            let score = self.score_document(doc, &query_terms);
            if score > 0.0 {
                scores.push((doc.path.clone(), score));
            }
        }

        // Sort by score descending
        scores.sort_by(|left_score, right_score| {
            right_score
                .1
                .partial_cmp(&left_score.1)
                .unwrap_or(Ordering::Equal)
        });
        scores.truncate(top_k);

        scores
    }

    /// Score a document against query terms using BM25
    fn score_document(&self, doc: &Document, query_terms: &[String]) -> f32 {
        let mut score = 0.0;

        for term in query_terms {
            let term_freq = *doc.terms.get(term).unwrap_or(&0) as f32;
            if term_freq == 0.0 {
                continue;
            }

            let idf = self.idf_cache.get(term).copied().unwrap_or(0.0);
            let doc_len_norm = doc.length as f32 / self.avg_doc_length;

            // BM25 formula
            let numerator = term_freq * (TERM_SATURATION_K1 + 1.0);
            let denominator = TERM_SATURATION_K1.mul_add(
                LENGTH_NORM_B.mul_add(doc_len_norm, 1.0 - LENGTH_NORM_B),
                term_freq,
            );

            score += idf * (numerator / denominator);
        }
        score
    }

    /// Extract path components from :: separated terms
    fn extract_path_components(path: &str, stopwords: &HashSet<&str>, terms: &mut Vec<String>) {
        for component in path.split("::") {
            if component.len() > 2 && !stopwords.contains(component) {
                terms.push(component.to_string());
            }
        }
    }

    /// Tokenize text into terms with special token preservation and bigrams
    fn tokenize(text: &str) -> Vec<String> {
        let stopwords = Self::stopwords();
        let mut terms = Vec::default();
        let words: Vec<&str> = text.split_whitespace().collect();

        for word in &words {
            let lower = word.to_lowercase();

            let has_double_colon = lower.contains("::");
            let has_double_dash = lower.starts_with("--");
            let has_special = has_double_colon || has_double_dash;

            if has_special && lower.len() > 2 {
                terms.push(lower.clone());

                if has_double_colon {
                    Self::extract_path_components(&lower, stopwords, &mut terms);
                }
            }

            let clean: String = lower
                .chars()
                .filter(|character| character.is_alphanumeric() || *character == '_')
                .collect();

            if !clean.is_empty()
                && clean.len() > 2
                && !stopwords.contains(clean.as_str())
                && (!has_special || clean != lower)
            {
                terms.push(clean);
            }
        }

        for window in words.windows(2) {
            let first_word = window[0].to_lowercase();
            let second_word = window[1].to_lowercase();

            let clean0: String = first_word
                .chars()
                .filter(|character| character.is_alphanumeric() || *character == '_')
                .collect();
            let clean1: String = second_word
                .chars()
                .filter(|character| character.is_alphanumeric() || *character == '_')
                .collect();

            if clean0.len() > 2
                && clean1.len() > 2
                && !stopwords.contains(clean0.as_str())
                && !stopwords.contains(clean1.as_str())
            {
                terms.push(format!("{clean0}_{clean1}"));
            }
        }

        terms
    }

    /// Count term frequencies
    fn count_terms(terms: &[String]) -> HashMap<String, usize> {
        let mut freq = HashMap::default();
        for term in terms {
            *freq.entry(term.clone()).or_insert(0) += 1;
        }
        freq
    }

    /// Get the number of documents in the index
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if the index is empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}
