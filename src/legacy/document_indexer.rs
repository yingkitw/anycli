use anyhow::{Result, anyhow};
use crate::vector_store::{VectorStore, DocumentChunk};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceSource {
    pub name: String,
    pub url: String,
    pub source_type: SourceType,
    pub priority: u8, // 1-10, higher means more important
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    IBMCloudDocs,
    WatsonXDocs,
    CarbonDesignSystem,
    IBMCloudCLI,
    Custom,
}

pub struct DocumentIndexer {
    vector_store: VectorStore,
    reference_sources: Vec<ReferenceSource>,
}

impl DocumentIndexer {
    /// Create a new DocumentIndexer with vector store
    pub async fn new(qdrant_url: &str, collection_name: &str) -> Result<Self> {
        let vector_store = VectorStore::new(qdrant_url, collection_name).await?;
        
        // Default reference sources for IBM Cloud CLI
        let reference_sources = vec![
            ReferenceSource {
                name: "IBM Cloud CLI Reference".to_string(),
                url: "https://cloud.ibm.com/docs/cli".to_string(),
                source_type: SourceType::IBMCloudCLI,
                priority: 10,
            },
            ReferenceSource {
                name: "IBM Cloud CLI Commands".to_string(),
                url: "https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_cli".to_string(),
                source_type: SourceType::IBMCloudCLI,
                priority: 9,
            },
            ReferenceSource {
                name: "WatsonX API Documentation".to_string(),
                url: "https://www.ibm.com/docs/en/watsonx/saas?topic=tutorials-watsonx-apis-sdks".to_string(),
                source_type: SourceType::WatsonXDocs,
                priority: 8,
            },
            ReferenceSource {
                name: "IBM Cloud Code Engine CLI".to_string(),
                url: "https://cloud.ibm.com/docs/codeengine?topic=codeengine-cli".to_string(),
                source_type: SourceType::IBMCloudDocs,
                priority: 8,
            },
            ReferenceSource {
                name: "IBM Cloud Functions CLI".to_string(),
                url: "https://cloud.ibm.com/docs/openwhisk?topic=openwhisk-functions-cli".to_string(),
                source_type: SourceType::IBMCloudDocs,
                priority: 7,
            },
            ReferenceSource {
                name: "IBM Cloud Kubernetes Service CLI".to_string(),
                url: "https://cloud.ibm.com/docs/containers?topic=containers-kubernetes-service-cli".to_string(),
                source_type: SourceType::IBMCloudDocs,
                priority: 7,
            },
            ReferenceSource {
                name: "Carbon Design System".to_string(),
                url: "https://carbondesignsystem.com/".to_string(),
                source_type: SourceType::CarbonDesignSystem,
                priority: 5,
            },
        ];
        
        Ok(Self {
            vector_store,
            reference_sources,
        })
    }
    
    /// Add a custom reference source
    pub fn add_reference_source(&mut self, source: ReferenceSource) {
        self.reference_sources.push(source);
    }
    
    /// Index all reference sources
    pub async fn index_all_sources(&self) -> Result<usize> {
        println!("🚀 Starting to index {} reference sources...", self.reference_sources.len());
        
        let mut total_chunks = 0;
        
        for (i, source) in self.reference_sources.iter().enumerate() {
            println!("\n📚 [{}/{}] Indexing: {} (Priority: {})", 
                i + 1, self.reference_sources.len(), source.name, source.priority);
            
            match self.index_source(source).await {
                Ok(chunks) => {
                    total_chunks += chunks;
                    println!("✅ Successfully indexed {} chunks from {}", chunks, source.name);
                }
                Err(e) => {
                    println!("❌ Failed to index {}: {}", source.name, e);
                    // Continue with other sources even if one fails
                }
            }
            
            // Add delay between requests to be respectful to servers
            sleep(Duration::from_millis(1000)).await;
        }
        
        println!("\n🎉 Indexing complete! Total chunks indexed: {}", total_chunks);
        self.vector_store.get_collection_info().await?;
        
        Ok(total_chunks)
    }
    
    /// Index a specific reference source
    async fn index_source(&self, source: &ReferenceSource) -> Result<usize> {
        match source.source_type {
            SourceType::IBMCloudDocs | 
            SourceType::WatsonXDocs | 
            SourceType::IBMCloudCLI | 
            SourceType::CarbonDesignSystem => {
                self.index_web_source(source).await
            }
            SourceType::Custom => {
                // For custom sources, we might support local files or other formats
                self.index_custom_source(source).await
            }
        }
    }
    
    /// Index a web-based source
    async fn index_web_source(&self, source: &ReferenceSource) -> Result<usize> {
        let chunks = self.vector_store.index_webpage(&source.url).await?;
        
        // Add source-specific metadata to chunks if needed
        // This could be enhanced to update existing chunks with priority info
        
        Ok(chunks)
    }
    
    /// Index a custom source (local files, etc.)
    async fn index_custom_source(&self, source: &ReferenceSource) -> Result<usize> {
        // For now, treat custom sources as local file paths
        if Path::new(&source.url).exists() {
            self.index_local_file(&source.url, &source.name).await
        } else {
            Err(anyhow!("Custom source file not found: {}", source.url))
        }
    }
    
    /// Index a local file
    async fn index_local_file(&self, file_path: &str, source_name: &str) -> Result<usize> {
        let content = fs::read_to_string(file_path)?;
        
        // Split content into chunks (simple approach - can be enhanced)
        let chunks = self.split_text_into_chunks(&content, source_name, file_path)?;
        
        // Index each chunk
        for chunk in &chunks {
            self.vector_store.index_document(chunk).await?;
        }
        
        Ok(chunks.len())
    }
    
    /// Split text content into manageable chunks
    fn split_text_into_chunks(&self, content: &str, source_name: &str, source_path: &str) -> Result<Vec<DocumentChunk>> {
        let mut chunks = Vec::new();
        
        // Simple chunking strategy: split by paragraphs and limit size
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        
        for (i, paragraph) in paragraphs.iter().enumerate() {
            let text = paragraph.trim();
            if text.len() > 50 { // Skip very short paragraphs
                let mut metadata = HashMap::new();
                metadata.insert("source_name".to_string(), source_name.to_string());
                metadata.insert("chunk_index".to_string(), i.to_string());
                metadata.insert("type".to_string(), "text_chunk".to_string());
                
                let chunk_id = format!("{}_{}", 
                    source_path.replace(['/', '\\', ':', '.'], "_"), i);
                
                chunks.push(DocumentChunk {
                    id: chunk_id,
                    content: text.to_string(),
                    source: source_path.to_string(),
                    metadata,
                });
            }
        }
        
        Ok(chunks)
    }
    
    /// Search for relevant context based on query
    pub async fn search_context(&self, query: &str, limit: u64) -> Result<Vec<DocumentChunk>> {
        self.vector_store.search(query, limit).await
    }
    
    /// Get enhanced context for IBM Cloud CLI queries
    pub async fn get_cli_context(&self, query: &str) -> Result<String> {
        // Search for relevant documentation
        let relevant_chunks = self.search_context(query, 5).await?;
        
        if relevant_chunks.is_empty() {
            return Ok(String::new());
        }
        
        // Format context for use in prompts
        let mut context = String::from("\n=== RELEVANT DOCUMENTATION ===\n");
        
        for (i, chunk) in relevant_chunks.iter().enumerate() {
            context.push_str(&format!("\n[{}] Source: {}\n", i + 1, chunk.source));
            
            // Add metadata if available
            if let Some(doc_type) = chunk.metadata.get("type") {
                context.push_str(&format!("Type: {}\n", doc_type));
            }
            
            context.push_str(&format!("Content: {}\n", chunk.content));
            
            if i < relevant_chunks.len() - 1 {
                context.push_str("\n---\n");
            }
        }
        
        context.push_str("\n=== END DOCUMENTATION ===\n");
        
        Ok(context)
    }
    
    /// Load reference sources from a configuration file
    pub fn load_sources_from_config(&mut self, config_path: &str) -> Result<()> {
        if Path::new(config_path).exists() {
            let config_content = fs::read_to_string(config_path)?;
            let sources: Vec<ReferenceSource> = serde_json::from_str(&config_content)?;
            let sources_len = sources.len();
            self.reference_sources.extend(sources);
            println!("📋 Loaded {} additional reference sources from config", sources_len);
        }
        Ok(())
    }
    
    /// Save current reference sources to a configuration file
    pub fn save_sources_to_config(&self, config_path: &str) -> Result<()> {
        let config_content = serde_json::to_string_pretty(&self.reference_sources)?;
        fs::write(config_path, config_content)?;
        println!("💾 Saved {} reference sources to config", self.reference_sources.len());
        Ok(())
    }
    
    /// Get statistics about indexed content
    pub async fn get_indexing_stats(&self) -> Result<()> {
        println!("📊 Document Indexer Statistics:");
        println!("   Reference sources configured: {}", self.reference_sources.len());
        
        // Group by source type
        let mut type_counts = HashMap::new();
        for source in &self.reference_sources {
            let count = type_counts.entry(format!("{:?}", source.source_type)).or_insert(0);
            *count += 1;
        }
        
        for (source_type, count) in type_counts {
            println!("   {}: {} sources", source_type, count);
        }
        
        // Get vector store info
        self.vector_store.get_collection_info().await?;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_document_indexer_creation() {
        let indexer_result = DocumentIndexer::new("http://localhost:6333", "test_indexer").await;
        if indexer_result.is_err() {
            println!("Skipping test: Qdrant not available");
            return;
        }
        
        let indexer = indexer_result.unwrap();
        assert!(!indexer.reference_sources.is_empty());
        assert!(indexer.reference_sources.iter().any(|s| matches!(s.source_type, SourceType::IBMCloudCLI)));
    }
    
    #[test]
    fn test_text_chunking() {
        let content = "This is paragraph one.\n\nThis is paragraph two with more content.\n\nShort.\n\nThis is a longer paragraph with sufficient content for indexing.";
        
        // This would need a proper DocumentIndexer instance in a real test
        // For now, we'll test the logic conceptually
        let paragraphs: Vec<&str> = content.split("\n\n").collect();
        let valid_paragraphs: Vec<&str> = paragraphs.into_iter()
            .filter(|p| p.trim().len() > 50)
            .collect();
            
        assert_eq!(valid_paragraphs.len(), 2); // Should filter out short paragraphs
    }
}