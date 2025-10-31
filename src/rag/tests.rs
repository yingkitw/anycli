//! Snapshot tests for RAG components

#[cfg(test)]
mod snapshot_tests {
    use super::{LocalVectorStore, LocalDocumentIndexer, LocalRAGEngine};
    use crate::core::{VectorStore, VectorDocument, SearchConfig, Document, RAGQuery, DocumentIndexer, RAGEngine};
    use serde_json::json;
    use std::sync::Arc;
    use insta::assert_yaml_snapshot;

    #[tokio::test]
    async fn test_vector_store_operations_snapshot() {
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
        
        assert_yaml_snapshot!("vector_store_search_results", results.documents);
    }

    #[tokio::test]
    async fn test_document_indexing_snapshot() {
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
        
        assert_yaml_snapshot!("indexing_result", result);
    }

    #[tokio::test]
    async fn test_rag_engine_snapshot() {
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
        
        assert_yaml_snapshot!("rag_retrieve_result", {
            ".documents[].score" => insta::rounded_redaction(2),
        }, result);
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
        
        assert_yaml_snapshot!("rag_context", context);
    }
}
