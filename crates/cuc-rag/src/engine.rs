//! RAG engine implementation

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

use cuc_core::{
    RAGEngine, RAGQuery, RAGResult,
    VectorStore, VectorDocument, SearchConfig,
    DocumentIndexer, Document,
    Error, Result,
};

/// Local RAG engine implementation
pub struct LocalRAGEngine<V: VectorStore, D: DocumentIndexer> {
    vector_store: Arc<V>,
    document_indexer: Arc<D>,
    initialized: bool,
}

impl<V: VectorStore, D: DocumentIndexer> LocalRAGEngine<V, D> {
    /// Create a new local RAG engine
    pub fn new(vector_store: Arc<V>, document_indexer: Arc<D>) -> Self {
        Self {
            vector_store,
            document_indexer,
            initialized: false,
        }
    }

    /// Add basic IBM Cloud CLI knowledge
    pub async fn add_basic_knowledge(&self) -> Result<()> {
        let basic_knowledge = vec![
            (
                "IBM Cloud CLI is a command-line interface that provides a set of commands for managing IBM Cloud resources. You can use it to create, configure, and manage IBM Cloud services from your terminal.",
                "IBM Cloud CLI Overview",
                "basic_knowledge"
            ),
            (
                "To install IBM Cloud CLI, you can download it from the IBM Cloud website or use package managers like Homebrew on macOS or apt-get on Ubuntu. After installation, use 'ibmcloud login' to authenticate.",
                "IBM Cloud CLI Installation",
                "installation_guide"
            ),
            (
                "Common IBM Cloud CLI commands include: 'ibmcloud login' for authentication, 'ibmcloud target' to set your target organization and space, 'ibmcloud resource groups' to list resource groups, and 'ibmcloud cf apps' to list Cloud Foundry applications.",
                "IBM Cloud CLI Commands",
                "command_reference"
            ),
            (
                "IBM Cloud CLI plugins extend the functionality of the CLI. You can install plugins using 'ibmcloud plugin install <plugin-name>'. Popular plugins include container-service, cloud-functions, and dev.",
                "IBM Cloud CLI Plugins",
                "plugin_guide"
            ),
            (
                "To manage Cloud Foundry applications with IBM Cloud CLI, use commands like 'ibmcloud cf push' to deploy apps, 'ibmcloud cf apps' to list apps, 'ibmcloud cf logs' to view logs, and 'ibmcloud cf delete' to remove apps.",
                "Cloud Foundry Management",
                "cf_commands"
            ),
            (
                "IBM Cloud CLI supports multiple output formats including JSON, table, and CSV. Use the '--output json' flag to get machine-readable output for scripting and automation.",
                "IBM Cloud CLI Output Formats",
                "output_formatting"
            ),
        ];

        let documents: Vec<Document> = basic_knowledge
            .into_iter()
            .enumerate()
            .map(|(i, (content, title, category))| Document {
                id: format!("basic_knowledge_{}", i),
                title: title.to_string(),
                content: content.to_string(),
                url: None,
                metadata: json!({
                    "category": category,
                    "type": "documentation",
                }),
            })
            .collect();

        self.document_indexer.index_documents(documents).await?;
        Ok(())
    }
}

#[async_trait]
impl<V: VectorStore + 'static, D: DocumentIndexer + 'static> RAGEngine for LocalRAGEngine<V, D> {
    async fn initialize(&mut self) -> Result<()> {
        if !self.vector_store.is_connected() {
            return Err(Error::RAGEngine("Vector store not connected".to_string()));
        }

        // Add basic knowledge
        self.add_basic_knowledge().await?;

        self.initialized = true;
        Ok(())
    }

    async fn retrieve(&self, query: &RAGQuery) -> Result<RAGResult> {
        if !self.initialized {
            return Err(Error::RAGEngine("RAG engine not initialized".to_string()));
        }

        let search_config = SearchConfig {
            top_k: query.top_k,
            score_threshold: query.score_threshold,
            filters: query.filters.as_ref().map(|f| {
                json!(f.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>())
            }),
        };

        let search_result = self.vector_store.search(&query.query, &search_config).await?;
        let context = self.build_context(&search_result.documents);

        Ok(RAGResult {
            documents: search_result.documents,
            context,
            metadata: Some(json!({
                "query": query.query,
                "top_k": query.top_k,
                "results_count": search_result.total,
            })),
        })
    }

    fn build_context(&self, documents: &[VectorDocument]) -> String {
        if documents.is_empty() {
            return String::new();
        }

        let mut context = String::from("Relevant IBM Cloud CLI documentation:\n\n");

        for (i, doc) in documents.iter().enumerate() {
            context.push_str(&format!("{}. ", i + 1));

            if let Some(title) = doc.metadata.get("title") {
                if let Some(title_str) = title.as_str() {
                    context.push_str(&format!("[{}] ", title_str));
                }
            }

            context.push_str(&doc.content);
            context.push_str("\n\n");
        }

        context
    }

    async fn enhance_prompt(&self, prompt: &str, query: &RAGQuery) -> Result<String> {
        let rag_result = self.retrieve(query).await?;

        let mut enhanced = String::new();
        enhanced.push_str(&rag_result.context);
        enhanced.push_str("\n---\n\n");
        enhanced.push_str("Based on the above documentation, ");
        enhanced.push_str(prompt);

        Ok(enhanced)
    }

    async fn stats(&self) -> Result<serde_json::Value> {
        let vector_count = self.vector_store.count().await?;
        let indexer_stats = self.document_indexer.stats().await?;

        Ok(json!({
            "initialized": self.initialized,
            "vector_store_count": vector_count,
            "indexer_stats": indexer_stats,
        }))
    }

    fn is_ready(&self) -> bool {
        self.initialized && self.vector_store.is_connected()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_store::LocalVectorStore;
    use crate::document_indexer::LocalDocumentIndexer;

    #[tokio::test]
    async fn test_rag_engine() {
        let mut store = LocalVectorStore::new();
        store.connect().await.unwrap();
        let store = Arc::new(store);

        let indexer = Arc::new(LocalDocumentIndexer::new(store.clone()));
        let mut engine = LocalRAGEngine::new(store, indexer);

        engine.initialize().await.unwrap();
        assert!(engine.is_ready());

        let query = RAGQuery {
            query: "IBM Cloud CLI commands".to_string(),
            top_k: 3,
            score_threshold: Some(0.1),
            filters: None,
        };

        let result = engine.retrieve(&query).await.unwrap();
        assert!(!result.documents.is_empty());
        assert!(!result.context.is_empty());
    }
}
