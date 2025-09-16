use anyhow::{Result, anyhow};
use crate::document_indexer::DocumentIndexer;
use crate::vector_store::DocumentChunk;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RAGConfig {
    pub max_context_chunks: u64,
    pub similarity_threshold: f32,
    pub context_window_size: usize,
    pub enable_query_expansion: bool,
    pub prioritize_recent: bool,
}

impl Default for RAGConfig {
    fn default() -> Self {
        Self {
            max_context_chunks: 5,
            similarity_threshold: 0.7,
            context_window_size: 2000,
            enable_query_expansion: true,
            prioritize_recent: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RAGContext {
    pub relevant_chunks: Vec<DocumentChunk>,
    pub context_summary: String,
    pub confidence_score: f32,
    pub sources_used: Vec<String>,
}

pub struct RAGEngine {
    document_indexer: DocumentIndexer,
    config: RAGConfig,
}

impl RAGEngine {
    /// Create a new RAG engine with document indexer
    pub async fn new(qdrant_url: &str, collection_name: &str) -> Result<Self> {
        let document_indexer = DocumentIndexer::new(qdrant_url, collection_name).await?;
        
        Ok(Self {
            document_indexer,
            config: RAGConfig::default(),
        })
    }
    
    /// Create RAG engine with custom configuration
    pub async fn with_config(qdrant_url: &str, collection_name: &str, config: RAGConfig) -> Result<Self> {
        let document_indexer = DocumentIndexer::new(qdrant_url, collection_name).await?;
        
        Ok(Self {
            document_indexer,
            config,
        })
    }
    
    /// Initialize the RAG system by indexing reference sources
    pub async fn initialize(&self) -> Result<()> {
        println!("🔧 Initializing RAG system...");
        
        let chunks_indexed = self.document_indexer.index_all_sources().await?;
        
        if chunks_indexed == 0 {
            println!("⚠️  Warning: No documents were indexed. RAG functionality will be limited.");
        } else {
            println!("✅ RAG system initialized with {} document chunks", chunks_indexed);
        }
        
        Ok(())
    }
    
    /// Retrieve relevant context for a query
    pub async fn retrieve_context(&self, query: &str) -> Result<RAGContext> {
        println!("🔍 Retrieving context for query: {}", query);
        
        // Expand query if enabled
        let search_query = if self.config.enable_query_expansion {
            self.expand_query(query)
        } else {
            query.to_string()
        };
        
        // Search for relevant chunks
        let mut relevant_chunks = self.document_indexer
            .search_context(&search_query, self.config.max_context_chunks)
            .await?;
        
        // Filter by similarity threshold if needed
        // Note: This would require similarity scores from the vector store
        // For now, we'll trust the vector store's ranking
        
        // Sort by priority if we have source metadata
        relevant_chunks.sort_by(|a, b| {
            let priority_a = self.get_source_priority(&a.source);
            let priority_b = self.get_source_priority(&b.source);
            priority_b.cmp(&priority_a) // Higher priority first
        });
        
        // Create context summary
        let context_summary = self.create_context_summary(&relevant_chunks, query)?;
        
        // Calculate confidence score
        let confidence_score = self.calculate_confidence_score(&relevant_chunks, query);
        
        // Extract unique sources
        let sources_used: Vec<String> = relevant_chunks
            .iter()
            .map(|chunk| chunk.source.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        
        let rag_context = RAGContext {
            relevant_chunks,
            context_summary,
            confidence_score,
            sources_used,
        };
        
        println!("📋 Retrieved {} relevant chunks with confidence: {:.2}", 
            rag_context.relevant_chunks.len(), rag_context.confidence_score);
        
        Ok(rag_context)
    }
    
    /// Expand query with related terms for better retrieval
    fn expand_query(&self, query: &str) -> String {
        let mut expanded = query.to_string();
        
        // Add IBM Cloud CLI specific expansions
        if query.contains("ibmcloud") || query.contains("ic") {
            expanded.push_str(" CLI command line interface");
        }
        
        if query.contains("login") || query.contains("auth") {
            expanded.push_str(" authentication credentials API key");
        }
        
        if query.contains("deploy") {
            expanded.push_str(" deployment application service");
        }
        
        if query.contains("function") || query.contains("serverless") {
            expanded.push_str(" OpenWhisk Cloud Functions");
        }
        
        if query.contains("kubernetes") || query.contains("k8s") {
            expanded.push_str(" container cluster IKS");
        }
        
        expanded
    }
    
    /// Get priority score for a source (higher is better)
    fn get_source_priority(&self, source: &str) -> u8 {
        if source.contains("cloud.ibm.com/docs/cli") {
            return 10; // Highest priority for CLI docs
        }
        if source.contains("watsonx") {
            return 8;
        }
        if source.contains("cloud.ibm.com") {
            return 7;
        }
        if source.contains("carbondesignsystem.com") {
            return 5;
        }
        3 // Default priority
    }
    
    /// Create a formatted context summary for use in prompts
    fn create_context_summary(&self, chunks: &[DocumentChunk], query: &str) -> Result<String> {
        if chunks.is_empty() {
            return Ok(String::new());
        }
        
        let mut summary = String::new();
        summary.push_str(&format!("\n=== RELEVANT CONTEXT FOR: {} ===\n", query));
        
        let mut current_length = 0;
        let max_length = self.config.context_window_size;
        
        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_text = format!("\n[{}] Source: {}\n{}", 
                i + 1, 
                self.extract_source_name(&chunk.source),
                chunk.content
            );
            
            if current_length + chunk_text.len() > max_length {
                summary.push_str("\n[...] (Additional context truncated to fit window)\n");
                break;
            }
            
            summary.push_str(&chunk_text);
            current_length += chunk_text.len();
            
            if i < chunks.len() - 1 {
                summary.push_str("\n---\n");
            }
        }
        
        summary.push_str("\n=== END CONTEXT ===\n");
        
        Ok(summary)
    }
    
    /// Extract a readable source name from URL or path
    fn extract_source_name(&self, source: &str) -> String {
        if source.starts_with("http") {
            // Extract domain and path info
            if let Ok(url) = url::Url::parse(source) {
                if let Some(host) = url.host_str() {
                    let path = url.path();
                    return format!("{}{}", host, path);
                }
            }
        }
        
        // For local files or other sources
        source.split('/').last().unwrap_or(source).to_string()
    }
    
    /// Calculate confidence score based on retrieved chunks
    fn calculate_confidence_score(&self, chunks: &[DocumentChunk], query: &str) -> f32 {
        if chunks.is_empty() {
            return 0.0;
        }
        
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        let mut total_score = 0.0;
        
        for chunk in chunks {
            let chunk_content_lower = chunk.content.to_lowercase();
            let chunk_words: Vec<&str> = chunk_content_lower.split_whitespace().collect();
            
            // Simple word overlap scoring
            let overlap_count = query_words.iter()
                .filter(|word| chunk_words.contains(word))
                .count();
            
            let overlap_ratio = overlap_count as f32 / query_words.len() as f32;
            
            // Boost score based on source priority
            let priority_boost = self.get_source_priority(&chunk.source) as f32 / 10.0;
            
            total_score += overlap_ratio * priority_boost;
        }
        
        // Average and normalize to 0-1 range
        let avg_score = total_score / chunks.len() as f32;
        avg_score.min(1.0)
    }
    
    /// Generate an enhanced prompt with RAG context
    pub async fn enhance_prompt(&self, original_prompt: &str, query: &str) -> Result<String> {
        let rag_context = self.retrieve_context(query).await?;
        
        if rag_context.relevant_chunks.is_empty() {
            println!("ℹ️  No relevant context found, using original prompt");
            return Ok(original_prompt.to_string());
        }
        
        let enhanced_prompt = format!(
            "{context}\n\nBased on the above documentation context, please {original_prompt}\n\nEnsure your response is accurate and references the provided documentation when relevant.",
            context = rag_context.context_summary,
            original_prompt = original_prompt
        );
        
        println!("🚀 Enhanced prompt with {} sources (confidence: {:.2})", 
            rag_context.sources_used.len(), rag_context.confidence_score);
        
        Ok(enhanced_prompt)
    }
    
    /// Get RAG system statistics
    pub async fn get_stats(&self) -> Result<HashMap<String, String>> {
        let mut stats = HashMap::new();
        
        stats.insert("max_context_chunks".to_string(), self.config.max_context_chunks.to_string());
        stats.insert("similarity_threshold".to_string(), self.config.similarity_threshold.to_string());
        stats.insert("context_window_size".to_string(), self.config.context_window_size.to_string());
        stats.insert("query_expansion_enabled".to_string(), self.config.enable_query_expansion.to_string());
        
        // Get indexer stats
        self.document_indexer.get_indexing_stats().await?;
        
        Ok(stats)
    }
    
    /// Update RAG configuration
    pub fn update_config(&mut self, config: RAGConfig) {
        self.config = config;
        println!("🔧 RAG configuration updated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_rag_engine_creation() {
        let rag_result = RAGEngine::new("http://localhost:6333", "test_rag").await;
        if rag_result.is_err() {
            println!("Skipping test: Qdrant not available");
            return;
        }
        
        let rag = rag_result.unwrap();
        assert_eq!(rag.config.max_context_chunks, 5);
    }
    
    #[test]
    fn test_query_expansion() {
        let config = RAGConfig::default();
        let rag = RAGEngine {
            document_indexer: unsafe { std::mem::zeroed() }, // This is just for testing
            config,
        };
        
        let expanded = rag.expand_query("ibmcloud login");
        assert!(expanded.contains("CLI"));
        assert!(expanded.contains("authentication"));
    }
    
    #[test]
    fn test_source_priority() {
        let config = RAGConfig::default();
        let rag = RAGEngine {
            document_indexer: unsafe { std::mem::zeroed() }, // This is just for testing
            config,
        };
        
        assert_eq!(rag.get_source_priority("https://cloud.ibm.com/docs/cli"), 10);
        assert_eq!(rag.get_source_priority("https://watsonx.ai/docs"), 8);
        assert_eq!(rag.get_source_priority("https://example.com"), 3);
    }
    
    #[test]
    fn test_confidence_calculation() {
        let config = RAGConfig::default();
        let rag = RAGEngine {
            document_indexer: unsafe { std::mem::zeroed() }, // This is just for testing
            config,
        };
        
        let chunks = vec![
            DocumentChunk {
                id: "test1".to_string(),
                content: "ibmcloud login with API key".to_string(),
                source: "https://cloud.ibm.com/docs/cli".to_string(),
                metadata: std::collections::HashMap::new(),
            }
        ];
        
        let score = rag.calculate_confidence_score(&chunks, "ibmcloud login");
        assert!(score > 0.0);
        assert!(score <= 1.0);
    }
}