//! Document indexer implementations

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::core::{
    DocumentIndexer, Document, IndexingResult, IndexingConfig,
    VectorStore, VectorDocument,
    Error, Result,
};

/// Local document indexer that works with any VectorStore
pub struct LocalDocumentIndexer<V: VectorStore> {
    vector_store: Arc<V>,
    config: IndexingConfig,
}

impl<V: VectorStore> LocalDocumentIndexer<V> {
    /// Create a new local document indexer
    pub fn new(vector_store: Arc<V>) -> Self {
        Self {
            vector_store,
            config: IndexingConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(vector_store: Arc<V>, config: IndexingConfig) -> Self {
        Self {
            vector_store,
            config,
        }
    }

    /// Chunk a document into smaller pieces
    fn chunk_document(&self, content: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut start = 0;

        while start < chars.len() {
            let end = (start + self.config.chunk_size).min(chars.len());
            let chunk: String = chars[start..end].iter().collect();
            chunks.push(chunk);

            if end >= chars.len() {
                break;
            }

            start = end - self.config.chunk_overlap;
        }

        chunks
    }
}

#[async_trait]
impl<V: VectorStore + 'static> DocumentIndexer for LocalDocumentIndexer<V> {
    async fn index_document(&self, document: Document) -> Result<IndexingResult> {
        let chunks = self.chunk_document(&document.content);
        let mut documents_indexed = 0;
        let mut documents_failed = 0;
        let mut errors = Vec::new();

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_id = format!("{}_{}", document.id, i);
            let mut metadata = document.metadata.clone();
            metadata["chunk_index"] = json!(i);
            metadata["total_chunks"] = json!(chunks.len());
            metadata["title"] = json!(document.title);
            if let Some(ref url) = document.url {
                metadata["url"] = json!(url);
            }

            let vector_doc = VectorDocument {
                id: chunk_id,
                content: chunk.clone(),
                embedding: None,
                metadata,
                score: None,
            };

            match self.vector_store.store(vector_doc).await {
                Ok(_) => documents_indexed += 1,
                Err(e) => {
                    documents_failed += 1;
                    errors.push(format!("Failed to store chunk {}: {}", i, e));
                }
            }
        }

        Ok(IndexingResult {
            documents_indexed,
            documents_failed,
            errors,
        })
    }

    async fn index_documents(&self, documents: Vec<Document>) -> Result<IndexingResult> {
        let mut total_indexed = 0;
        let mut total_failed = 0;
        let mut all_errors = Vec::new();

        for document in documents {
            match self.index_document(document).await {
                Ok(result) => {
                    total_indexed += result.documents_indexed;
                    total_failed += result.documents_failed;
                    all_errors.extend(result.errors);
                }
                Err(e) => {
                    total_failed += 1;
                    all_errors.push(format!("Failed to index document: {}", e));
                }
            }
        }

        Ok(IndexingResult {
            documents_indexed: total_indexed,
            documents_failed: total_failed,
            errors: all_errors,
        })
    }

    async fn index_from_url(&self, _url: &str) -> Result<IndexingResult> {
        // Placeholder - would require web scraping implementation
        Err(Error::DocumentIndexer("URL indexing not yet implemented".to_string()))
    }

    async fn index_from_urls(&self, urls: Vec<String>) -> Result<IndexingResult> {
        let mut total_indexed = 0;
        let mut total_failed = 0;
        let mut all_errors = Vec::new();

        for url in urls {
            match self.index_from_url(&url).await {
                Ok(result) => {
                    total_indexed += result.documents_indexed;
                    total_failed += result.documents_failed;
                    all_errors.extend(result.errors);
                }
                Err(e) => {
                    total_failed += 1;
                    all_errors.push(format!("Failed to index URL {}: {}", url, e));
                }
            }
        }

        Ok(IndexingResult {
            documents_indexed: total_indexed,
            documents_failed: total_failed,
            errors: all_errors,
        })
    }

    async fn index_from_file(&self, path: &str) -> Result<IndexingResult> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| Error::Io(e))?;

        let document = Document {
            id: Uuid::new_v4().to_string(),
            title: path.to_string(),
            content,
            url: None,
            metadata: json!({
                "source": "file",
                "path": path,
            }),
        };

        self.index_document(document).await
    }

    async fn stats(&self) -> Result<serde_json::Value> {
        let count = self.vector_store.count().await?;
        Ok(json!({
            "total_documents": count,
            "chunk_size": self.config.chunk_size,
            "chunk_overlap": self.config.chunk_overlap,
        }))
    }
}

/// Web document indexer with scraping capabilities
pub struct WebDocumentIndexer<V: VectorStore> {
    local_indexer: LocalDocumentIndexer<V>,
}

impl<V: VectorStore> WebDocumentIndexer<V> {
    pub fn new(vector_store: Arc<V>) -> Self {
        Self {
            local_indexer: LocalDocumentIndexer::new(vector_store),
        }
    }
}

#[async_trait]
impl<V: VectorStore + 'static> DocumentIndexer for WebDocumentIndexer<V> {
    async fn index_document(&self, document: Document) -> Result<IndexingResult> {
        self.local_indexer.index_document(document).await
    }

    async fn index_documents(&self, documents: Vec<Document>) -> Result<IndexingResult> {
        self.local_indexer.index_documents(documents).await
    }

    async fn index_from_url(&self, url: &str) -> Result<IndexingResult> {
        // TODO: Implement web scraping with scraper crate
        // For now, delegate to local indexer
        self.local_indexer.index_from_url(url).await
    }

    async fn index_from_urls(&self, urls: Vec<String>) -> Result<IndexingResult> {
        self.local_indexer.index_from_urls(urls).await
    }

    async fn index_from_file(&self, path: &str) -> Result<IndexingResult> {
        self.local_indexer.index_from_file(path).await
    }

    async fn stats(&self) -> Result<serde_json::Value> {
        self.local_indexer.stats().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rag::LocalVectorStore;

    #[tokio::test]
    async fn test_document_indexing() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();
        let store = Arc::new(store);

        let indexer = LocalDocumentIndexer::new(store.clone());

        let document = Document {
            id: "test_doc".to_string(),
            title: "Test Document".to_string(),
            content: "This is a test document for indexing. It contains some text.".to_string(),
            url: None,
            metadata: json!({"type": "test"}),
        };

        let result = indexer.index_document(document).await.unwrap();
        assert!(result.documents_indexed > 0);
        assert_eq!(result.documents_failed, 0);

        let count = store.count().await.unwrap();
        assert!(count > 0);
    }
}
