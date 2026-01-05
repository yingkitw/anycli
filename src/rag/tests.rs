//! Tests for RAG components

#[cfg(test)]
mod tests {
    use crate::rag::{LocalVectorStore, LocalDocumentIndexer, LocalRAGEngine};
    use crate::core::{VectorStore, VectorDocument, SearchConfig, Document, RAGQuery, DocumentIndexer, RAGEngine};
    use serde_json::json;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_vector_store_operations() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();

        let doc = VectorDocument {
            id: "test_doc_1".to_string(),
            content: "IBM Cloud CLI is a powerful command-line tool".to_string(),
            embedding: None,
            metadata: json!({"type": "documentation", "category": "cli"}),
            score: None,
        };

        store.store(doc.clone()).await.unwrap();
        
        let config = SearchConfig {
            top_k: 5,
            score_threshold: Some(0.1),
            filters: None,
        };

        let results = store.search("IBM Cloud CLI", &config).await.unwrap();
        
        assert!(!results.documents.is_empty(), "Should find at least one document");
        assert_eq!(results.documents[0].id, "test_doc_1", "Should find the stored document");
        assert!(results.documents[0].content.contains("IBM Cloud CLI"), 
            "Document content should contain search query");
    }

    #[tokio::test]
    async fn test_document_indexing() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();
        let store = Arc::new(store);

        let indexer = LocalDocumentIndexer::new(store.clone());

        let document = Document {
            id: "doc_1".to_string(),
            title: "IBM Cloud CLI Guide".to_string(),
            content: "This is a comprehensive guide to using IBM Cloud CLI. It covers installation, configuration, and basic commands.".to_string(),
            url: Some("https://cloud.ibm.com/docs/cli".to_string()),
            metadata: json!({"type": "guide", "version": "1.0"}),
        };

        let result = indexer.index_document(document).await.unwrap();
        
        assert_eq!(result.documents_indexed, 1, "Should index 1 document");
        assert_eq!(result.documents_failed, 0, "Should have no failures");
        assert!(result.errors.is_empty(), "Should have no errors");
    }

    #[tokio::test]
    #[ignore] // Requires full RAG engine initialization
    async fn test_rag_engine_retrieve() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();
        let store = Arc::new(store);

        let indexer = Arc::new(LocalDocumentIndexer::new(store.clone()));
        let mut engine = LocalRAGEngine::new(store, indexer);

        engine.initialize().await.unwrap();

        let query = RAGQuery {
            query: "How to use IBM Cloud CLI".to_string(),
            top_k: 3,
            score_threshold: Some(0.1),
            filters: None,
        };

        let result = engine.retrieve(&query).await.unwrap();
        
        assert!(!result.documents.is_empty(), "Should retrieve at least one document");
        assert!(!result.context.is_empty(), "Should have context");
        assert!(result.metadata.is_some(), "Should have metadata");
    }

    #[tokio::test]
    async fn test_rag_context_building() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();
        let store = Arc::new(store);

        let indexer = Arc::new(LocalDocumentIndexer::new(store.clone()));
        let mut engine = LocalRAGEngine::new(store, indexer);

        engine.initialize().await.unwrap();

        let docs = vec![
            VectorDocument {
                id: "1".to_string(),
                content: "IBM Cloud CLI commands".to_string(),
                embedding: None,
                metadata: json!({"title": "Commands"}),
                score: Some(0.9),
            },
            VectorDocument {
                id: "2".to_string(),
                content: "Installation guide".to_string(),
                embedding: None,
                metadata: json!({"title": "Installation"}),
                score: Some(0.8),
            },
        ];

        let context = engine.build_context(&docs);
        
        assert!(!context.is_empty(), "Context should not be empty");
        assert!(context.contains("IBM Cloud CLI commands"), 
            "Context should contain document content");
        assert!(context.contains("Installation guide"), 
            "Context should contain second document content");
    }
}
