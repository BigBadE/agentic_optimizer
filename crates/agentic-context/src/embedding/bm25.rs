//! BM25 keyword search implementation for file ranking.

use std::collections::HashMap;
use std::path::PathBuf;

/// BM25 parameters
const K1: f32 = 1.5;  // Term frequency saturation parameter
const B: f32 = 0.75;  // Length normalization parameter

/// Document in the BM25 index
#[derive(Debug, Clone)]
struct Document {
    path: PathBuf,
    terms: HashMap<String, usize>,  // term -> frequency
    length: usize,  // total terms in document
}

/// BM25 search index
pub struct BM25Index {
    documents: Vec<Document>,
    avg_doc_length: f32,
    idf_cache: HashMap<String, f32>,  // term -> IDF score
}

impl BM25Index {
    /// Create a new empty BM25 index
    #[must_use]
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            avg_doc_length: 0.0,
            idf_cache: HashMap::new(),
        }
    }

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
        let total_length: usize = self.documents.iter().map(|d| d.length).sum();
        self.avg_doc_length = total_length as f32 / self.documents.len() as f32;

        // Compute IDF for all terms
        let mut doc_freq: HashMap<String, usize> = HashMap::new();
        for doc in &self.documents {
            for term in doc.terms.keys() {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }
        }

        let num_docs = self.documents.len() as f32;
        for (term, df) in doc_freq {
            let idf = ((num_docs - df as f32 + 0.5) / (df as f32 + 0.5) + 1.0).ln();
            self.idf_cache.insert(term, idf);
        }
    }

    /// Search for documents matching the query
    pub fn search(&self, query: &str, top_k: usize) -> Vec<(PathBuf, f32)> {
        let query_terms = Self::tokenize(query);
        let mut scores: Vec<(PathBuf, f32)> = Vec::new();

        for doc in &self.documents {
            let score = self.score_document(doc, &query_terms);
            if score > 0.0 {
                scores.push((doc.path.clone(), score));
            }
        }

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(top_k);

        scores
    }

    /// Score a document against query terms using BM25
    fn score_document(&self, doc: &Document, query_terms: &[String]) -> f32 {
        let mut score = 0.0;

        for term in query_terms {
            let tf = *doc.terms.get(term).unwrap_or(&0) as f32;
            if tf == 0.0 {
                continue;
            }

            let idf = self.idf_cache.get(term).copied().unwrap_or(0.0);
            let doc_len_norm = doc.length as f32 / self.avg_doc_length;
            
            // BM25 formula
            let numerator = tf * (K1 + 1.0);
            let denominator = tf + K1 * (1.0 - B + B * doc_len_norm);
            
            score += idf * (numerator / denominator);
        }

        score
    }

    /// Tokenize text into terms with special token preservation and bigrams
    fn tokenize(text: &str) -> Vec<String> {
        let mut terms = Vec::new();
        let words: Vec<&str> = text.split_whitespace().collect();
        
        for word in &words {
            let lower = word.to_lowercase();
            
            // Preserve special tokens with punctuation
            if lower.contains("::") || lower.starts_with("--") || lower.starts_with('-') {
                if lower.len() > 2 {
                    terms.push(lower.clone());
                }
            }
            
            // Also add cleaned version
            let clean: String = lower
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            
            if !clean.is_empty() && clean.len() > 2 {
                terms.push(clean);
            }
        }
        
        // Add bigrams for common phrases
        for window in words.windows(2) {
            let w0 = window[0].to_lowercase();
            let w1 = window[1].to_lowercase();
            
            let clean0: String = w0.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
            let clean1: String = w1.chars().filter(|c| c.is_alphanumeric() || *c == '_').collect();
            
            if clean0.len() > 2 && clean1.len() > 2 {
                terms.push(format!("{}_{}", clean0, clean1));
            }
        }
        
        terms
    }

    /// Count term frequencies
    fn count_terms(terms: &[String]) -> HashMap<String, usize> {
        let mut freq = HashMap::new();
        for term in terms {
            *freq.entry(term.clone()).or_insert(0) += 1;
        }
        freq
    }

    /// Get the number of documents in the index
    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if the index is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

impl Default for BM25Index {
    fn default() -> Self {
        Self::new()
    }
}
