use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::local_vector_store::{LocalVectorStore, DocumentChunk};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceSource {
    pub name: String,
    pub url: String,
    pub source_type: SourceType,
    pub last_indexed: Option<String>,
    pub chunk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    Documentation,
    Tutorial,
    Reference,
    Example,
}

pub struct LocalDocumentIndexer {
    vector_store: LocalVectorStore,
    sources: Vec<ReferenceSource>,
}

impl LocalDocumentIndexer {
    /// Create a new document indexer with local vector store
    pub fn new(data_file: &str) -> Result<Self> {
        let vector_store = LocalVectorStore::new(data_file)?;
        let sources = Vec::new();
        
        Ok(Self {
            vector_store,
            sources,
        })
    }
    
    /// Add a reference source
    pub fn add_reference_source(&mut self, source: ReferenceSource) {
        // Remove existing source with same URL if it exists
        self.sources.retain(|s| s.url != source.url);
        self.sources.push(source);
    }
    
    /// Index IBM Cloud CLI documentation
    pub async fn index_ibm_cloud_docs(&mut self) -> Result<()> {
        println!("📚 Starting IBM Cloud CLI documentation indexing...");
        
        let ibm_docs = vec![
            ("IBM Cloud CLI Overview", "https://cloud.ibm.com/docs/cli"),
            ("IBM Cloud CLI Reference", "https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_cli"),
            ("Getting Started with IBM Cloud CLI", "https://cloud.ibm.com/docs/cli?topic=cli-getting-started"),
        ];
        
        let mut total_chunks = 0;
        
        for (name, url) in ibm_docs {
            println!("🔍 Indexing: {}", name);
            
            match self.vector_store.index_webpage(url).await {
                Ok(chunk_count) => {
                    total_chunks += chunk_count;
                    
                    let source = ReferenceSource {
                        name: name.to_string(),
                        url: url.to_string(),
                        source_type: SourceType::Documentation,
                        last_indexed: Some(chrono::Utc::now().to_rfc3339()),
                        chunk_count,
                    };
                    
                    self.add_reference_source(source);
                    println!("✅ Successfully indexed {} chunks from {}", chunk_count, name);
                }
                Err(e) => {
                    println!("❌ Failed to index {}: {}", name, e);
                }
            }
            
            // Add a small delay to be respectful to the server
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        }
        
        println!("🎉 Indexing complete! Total chunks indexed: {}", total_chunks);
        Ok(())
    }
    
    /// Index a custom webpage
    pub async fn index_webpage(&mut self, url: &str, name: &str, source_type: SourceType) -> Result<usize> {
        println!("🌐 Indexing webpage: {} ({})", name, url);
        
        let chunk_count = self.vector_store.index_webpage(url).await?;
        
        let source = ReferenceSource {
            name: name.to_string(),
            url: url.to_string(),
            source_type,
            last_indexed: Some(chrono::Utc::now().to_rfc3339()),
            chunk_count,
        };
        
        self.add_reference_source(source);
        
        println!("✅ Successfully indexed {} chunks from {}", chunk_count, name);
        Ok(chunk_count)
    }
    
    /// Index a text document directly
    pub fn index_text_document(&mut self, content: &str, source: &str, metadata: HashMap<String, String>) -> Result<()> {
        // Split content into chunks if it's too long
        let chunks = if content.len() > 1000 {
            self.split_text_into_chunks(content, source, &metadata)
        } else {
            vec![DocumentChunk {
                id: format!("{:x}", md5::compute(content.as_bytes())),
                content: content.to_string(),
                source: source.to_string(),
                metadata,
                embedding: Vec::new(),
            }]
        };
        
        for chunk in chunks {
            self.vector_store.index_document(&chunk)?;
        }
        
        Ok(())
    }
    
    /// Split long text into semantically coherent, minimal chunks for better matching
    fn split_text_into_chunks(&self, text: &str, source: &str, metadata: &HashMap<String, String>) -> Vec<DocumentChunk> {
        let mut chunks = Vec::new();
        let mut chunk_index = 0;
        
        // First, try to split by natural boundaries (paragraphs, then sentences)
        let paragraphs: Vec<&str> = text.split("\n\n").collect();
        
        for paragraph in paragraphs {
            let paragraph = paragraph.trim();
            if paragraph.is_empty() || paragraph.len() < 30 {
                continue; // Skip very short or empty paragraphs
            }
            
            // If paragraph is short enough, use it as a single chunk
            if paragraph.len() <= 400 {
                chunks.push(self.create_chunk(paragraph, source, metadata, chunk_index));
                chunk_index += 1;
            } else {
                // Split longer paragraphs by sentences, keeping semantic coherence
                let sentences = self.split_into_sentences(paragraph);
                let mut current_chunk = String::new();
                
                for sentence in sentences {
                    let sentence = sentence.trim();
                    if sentence.is_empty() {
                        continue;
                    }
                    
                    // Check if adding this sentence would exceed optimal chunk size
                    let potential_length = current_chunk.len() + sentence.len() + 2; // +2 for space and period
                    
                    if potential_length > 300 && !current_chunk.is_empty() {
                        // Save current chunk and start new one
                        chunks.push(self.create_chunk(&current_chunk, source, metadata, chunk_index));
                        chunk_index += 1;
                        current_chunk = sentence.to_string();
                    } else {
                        // Add sentence to current chunk
                        if !current_chunk.is_empty() {
                            current_chunk.push(' ');
                        }
                        current_chunk.push_str(sentence);
                    }
                }
                
                // Add the last chunk if it's not empty
                if !current_chunk.trim().is_empty() {
                    chunks.push(self.create_chunk(&current_chunk, source, metadata, chunk_index));
                    chunk_index += 1;
                }
            }
        }
        
        chunks
    }
    
    /// Helper function to create a document chunk
    fn create_chunk(&self, content: &str, source: &str, metadata: &HashMap<String, String>, index: usize) -> DocumentChunk {
        let mut chunk_metadata = metadata.clone();
        chunk_metadata.insert("chunk_index".to_string(), index.to_string());
        chunk_metadata.insert("chunk_size".to_string(), content.len().to_string());
        
        DocumentChunk {
            id: format!("{:x}-{}", md5::compute(source.as_bytes()), index),
            content: content.trim().to_string(),
            source: source.to_string(),
            metadata: chunk_metadata,
            embedding: Vec::new(),
        }
    }
    
    /// Smart sentence splitting that preserves semantic boundaries
    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current_sentence = String::new();
        let chars: Vec<char> = text.chars().collect();
        
        for (i, &ch) in chars.iter().enumerate() {
            current_sentence.push(ch);
            
            // Check for sentence endings
            if ch == '.' || ch == '!' || ch == '?' {
                // Look ahead to see if this is really the end of a sentence
                let is_sentence_end = if i + 1 < chars.len() {
                    let next_char = chars[i + 1];
                    // Not a sentence end if followed by lowercase letter or digit
                    !next_char.is_ascii_lowercase() && !next_char.is_ascii_digit()
                } else {
                    true // End of text
                };
                
                if is_sentence_end && current_sentence.trim().len() > 10 {
                    sentences.push(current_sentence.trim().to_string());
                    current_sentence.clear();
                }
            }
        }
        
        // Add any remaining text as the last sentence
        if !current_sentence.trim().is_empty() {
            sentences.push(current_sentence.trim().to_string());
        }
        
        sentences
    }
    
    /// Search for relevant context based on a query
    pub fn search_context(&self, query: &str, limit: usize) -> Result<Vec<DocumentChunk>> {
        self.vector_store.search(query, limit)
    }
    
    /// Filter chunks to keep only the most relevant ones for minimal matching
    fn filter_most_relevant_chunks(&self, chunks: &[DocumentChunk], query: &str) -> Vec<DocumentChunk> {
        if chunks.is_empty() {
            return Vec::new();
        }
        
        let query_lower = query.to_lowercase();
        let query_words: Vec<&str> = query_lower.split_whitespace().collect();
        
        // Score each chunk based on keyword overlap and content quality
        let mut scored_chunks: Vec<(f32, &DocumentChunk)> = chunks.iter()
            .map(|chunk| {
                let content_lower = chunk.content.to_lowercase();
                let content_words: Vec<&str> = content_lower.split_whitespace().collect();
                
                // Calculate keyword overlap score
                let overlap_count = query_words.iter()
                    .filter(|word| content_words.contains(word))
                    .count();
                
                let overlap_ratio = if query_words.is_empty() { 0.0 } else {
                    overlap_count as f32 / query_words.len() as f32
                };
                
                // Bonus for CLI-specific terms
                let cli_bonus = if content_lower.contains("ibmcloud") || 
                                  content_lower.contains("cli") || 
                                  content_lower.contains("command") {
                    0.2
                } else {
                    0.0
                };
                
                // Penalty for very long chunks (prefer concise, focused content)
                let length_penalty = if chunk.content.len() > 500 { -0.1 } else { 0.0 };
                
                let total_score = overlap_ratio + cli_bonus + length_penalty;
                (total_score, chunk)
            })
            .collect();
        
        // Sort by score (highest first)
        scored_chunks.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return only the top chunk for minimal matching, or top 2 if first score is low
        if scored_chunks.is_empty() {
            Vec::new()
        } else if scored_chunks.len() == 1 || scored_chunks[0].0 > 0.3 {
            vec![scored_chunks[0].1.clone()]
        } else {
            scored_chunks.into_iter().take(2).map(|(_, chunk)| chunk.clone()).collect()
        }
    }
    
    /// Get CLI context for a specific query with minimal matching
    pub async fn get_cli_context(&self, query: &str) -> Result<String> {
        // Use minimal matching - only get the most relevant chunk
        let relevant_chunks = self.search_context(query, 2)?;
        
        if relevant_chunks.is_empty() {
            return Ok(String::new());
        }
        
        // Filter chunks by relevance score for minimal matching
        let filtered_chunks = self.filter_most_relevant_chunks(&relevant_chunks, query);
        
        let mut context = String::from("\n--- Relevant Context ---\n");
        
        // Use only the most relevant chunks (minimal matching)
        for (i, chunk) in filtered_chunks.iter().enumerate() {
            context.push_str(&format!("\n{}. {}", i + 1, chunk.content));
            if i < filtered_chunks.len() - 1 {
                context.push_str("\n");
            }
        }
        
        context.push_str("\n--- End Context ---\n");
        Ok(context)
    }
    
    /// Get indexing statistics
    pub async fn get_indexing_stats(&self) -> Result<()> {
        self.vector_store.get_collection_info()?;
        
        println!("\n📋 Indexed Sources:");
        for source in &self.sources {
            println!("  • {} ({} chunks) - {}", 
                source.name, 
                source.chunk_count, 
                source.last_indexed.as_deref().unwrap_or("Never")
            );
        }
        
        Ok(())
    }
    
    /// Load sources from config file
    pub fn load_sources_from_config(&mut self, config_path: &str) -> Result<()> {
        if std::path::Path::new(config_path).exists() {
            let content = std::fs::read_to_string(config_path)?;
            self.sources = serde_json::from_str(&content)?;
        }
        Ok(())
    }
    
    /// Save sources to config file
    pub fn save_sources_to_config(&self, config_path: &str) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.sources)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[tokio::test]
    async fn test_document_indexer_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let indexer = LocalDocumentIndexer::new(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(indexer.sources.len(), 0);
    }
    
    #[test]
    fn test_text_chunking() {
        let temp_file = NamedTempFile::new().unwrap();
        let indexer = LocalDocumentIndexer::new(temp_file.path().to_str().unwrap()).unwrap();
        
        let long_text = "This is a very long text. ".repeat(50);
        let metadata = HashMap::new();
        let chunks = indexer.split_text_into_chunks(&long_text, "test_source", &metadata);
        
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.content.len() <= 1000);
            assert!(!chunk.content.is_empty());
        }
    }
}