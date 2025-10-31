//! Vector store implementations

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::core::{
    VectorStore, VectorDocument, SearchResult, SearchConfig,
    Error, Result,
};

/// Local in-memory vector store implementation
pub struct LocalVectorStore {
    documents: Arc<RwLock<HashMap<String, VectorDocument>>>,
    connected: bool,
}

impl LocalVectorStore {
    /// Create a new local vector store
    pub fn new() -> Self {
        Self {
            documents: Arc::new(RwLock::new(HashMap::new())),
            connected: false,
        }
    }

    /// Simple cosine similarity calculation
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot_product / (norm_a * norm_b)
    }

    /// Simple text-based similarity (for when embeddings are not available)
    fn text_similarity(query: &str, content: &str) -> f32 {
        let query_lower = query.to_lowercase();
        let content_lower = content.to_lowercase();

        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut matches = 0;

        for word in &query_words {
            if content_lower.contains(word) {
                matches += 1;
            }
        }

        if query_words.is_empty() {
            0.0
        } else {
            matches as f32 / query_words.len() as f32
        }
    }
}

impl Default for LocalVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for LocalVectorStore {
    async fn connect(&mut self) -> Result<()> {
        self.connected = true;
        Ok(())
    }

    async fn store(&self, document: VectorDocument) -> Result<String> {
        let id = document.id.clone();
        let mut docs = self.documents.write()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;
        docs.insert(id.clone(), document);
        Ok(id)
    }

    async fn store_batch(&self, documents: Vec<VectorDocument>) -> Result<Vec<String>> {
        let mut ids = Vec::new();
        let mut docs = self.documents.write()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;

        for document in documents {
            let id = document.id.clone();
            docs.insert(id.clone(), document);
            ids.push(id);
        }

        Ok(ids)
    }

    async fn search(&self, query: &str, config: &SearchConfig) -> Result<SearchResult> {
        let docs = self.documents.read()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;

        let mut results: Vec<VectorDocument> = docs
            .values()
            .map(|doc| {
                let score = Self::text_similarity(query, &doc.content);
                let mut doc_with_score = doc.clone();
                doc_with_score.score = Some(score);
                doc_with_score
            })
            .filter(|doc| {
                if let Some(threshold) = config.score_threshold {
                    doc.score.unwrap_or(0.0) >= threshold
                } else {
                    true
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score.unwrap_or(0.0).partial_cmp(&a.score.unwrap_or(0.0)).unwrap()
        });

        results.truncate(config.top_k);

        let total = results.len();

        Ok(SearchResult {
            documents: results,
            total,
        })
    }

    async fn search_by_vector(&self, vector: Vec<f32>, config: &SearchConfig) -> Result<SearchResult> {
        let docs = self.documents.read()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;

        let mut results: Vec<VectorDocument> = docs
            .values()
            .filter_map(|doc| {
                if let Some(ref embedding) = doc.embedding {
                    let score = Self::cosine_similarity(&vector, embedding);
                    let mut doc_with_score = doc.clone();
                    doc_with_score.score = Some(score);
                    Some(doc_with_score)
                } else {
                    None
                }
            })
            .filter(|doc| {
                if let Some(threshold) = config.score_threshold {
                    doc.score.unwrap_or(0.0) >= threshold
                } else {
                    true
                }
            })
            .collect();

        results.sort_by(|a, b| {
            b.score.unwrap_or(0.0).partial_cmp(&a.score.unwrap_or(0.0)).unwrap()
        });

        results.truncate(config.top_k);

        let total = results.len();

        Ok(SearchResult {
            documents: results,
            total,
        })
    }

    async fn get(&self, id: &str) -> Result<Option<VectorDocument>> {
        let docs = self.documents.read()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;
        Ok(docs.get(id).cloned())
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        let mut docs = self.documents.write()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;
        Ok(docs.remove(id).is_some())
    }

    async fn clear(&self) -> Result<()> {
        let mut docs = self.documents.write()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;
        docs.clear();
        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        let docs = self.documents.read()
            .map_err(|e| Error::VectorStore(format!("Lock error: {}", e)))?;
        Ok(docs.len())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }
}

/// Qdrant vector store implementation (placeholder for future implementation)
pub struct QdrantVectorStore {
    // TODO: Implement Qdrant integration
}

impl QdrantVectorStore {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for QdrantVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_vector_store() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();

        let doc = VectorDocument {
            id: "test1".to_string(),
            content: "IBM Cloud CLI is a command-line tool".to_string(),
            embedding: None,
            metadata: json!({"type": "test"}),
            score: None,
        };

        let id = store.store(doc).await.unwrap();
        assert_eq!(id, "test1");

        let count = store.count().await.unwrap();
        assert_eq!(count, 1);

        let retrieved = store.get("test1").await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_search() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();

        let doc1 = VectorDocument {
            id: "doc1".to_string(),
            content: "IBM Cloud CLI commands for resource management".to_string(),
            embedding: None,
            metadata: json!({}),
            score: None,
        };

        let doc2 = VectorDocument {
            id: "doc2".to_string(),
            content: "Kubernetes cluster management with IBM Cloud".to_string(),
            embedding: None,
            metadata: json!({}),
            score: None,
        };

        store.store(doc1).await.unwrap();
        store.store(doc2).await.unwrap();

        let config = SearchConfig {
            top_k: 2,
            score_threshold: Some(0.1),
            filters: None,
        };

        let results = store.search("IBM Cloud CLI", &config).await.unwrap();
        assert!(!results.documents.is_empty());
    }
}
