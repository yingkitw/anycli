use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use scraper::{Html, Selector};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use md5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    pub id: String,
    pub content: String,
    pub source: String,
    pub metadata: HashMap<String, String>,
    pub embedding: Vec<f32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct VectorStoreData {
    documents: Vec<DocumentChunk>,
    embedding_dimension: usize,
}

pub struct LocalVectorStore {
    data_file: String,
    embedding_dimension: usize,
    documents: Vec<DocumentChunk>,
}

impl LocalVectorStore {
    /// Create a new local vector store
    pub fn new(data_file: &str) -> Result<Self> {
        let embedding_dimension = 384;
        let documents = if Path::new(data_file).exists() {
            Self::load_from_file(data_file)?
        } else {
            Vec::new()
        };
        
        Ok(Self {
            data_file: data_file.to_string(),
            embedding_dimension,
            documents,
        })
    }
    
    /// Load documents from file
    fn load_from_file(data_file: &str) -> Result<Vec<DocumentChunk>> {
        let content = fs::read_to_string(data_file)?;
        let store_data: VectorStoreData = serde_json::from_str(&content)?;
        Ok(store_data.documents)
    }
    
    /// Save documents to file
    fn save_to_file(&self) -> Result<()> {
        let store_data = VectorStoreData {
            documents: self.documents.clone(),
            embedding_dimension: self.embedding_dimension,
        };
        
        let content = serde_json::to_string_pretty(&store_data)?;
        fs::write(&self.data_file, content)?;
        Ok(())
    }
    
    /// Generate simple hash-based embeddings for text
    fn generate_embeddings(&self, text: &str) -> Result<Vec<f32>> {
        let normalized_text = text.to_lowercase();
        let words: Vec<&str> = normalized_text.split_whitespace().collect();
        
        let mut embedding = vec![0.0; self.embedding_dimension];
        
        // Generate features based on word hashes and positions
        for (pos, word) in words.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            word.hash(&mut hasher);
            let hash = hasher.finish();
            
            // Use hash to determine feature indices
            let idx1 = (hash % self.embedding_dimension as u64) as usize;
            let idx2 = ((hash >> 16) % self.embedding_dimension as u64) as usize;
            let idx3 = ((hash >> 32) % self.embedding_dimension as u64) as usize;
            
            // Weight by position (earlier words get higher weight)
            let position_weight = 1.0 / (pos as f32 + 1.0);
            
            embedding[idx1] += position_weight;
            embedding[idx2] += position_weight * 0.7;
            embedding[idx3] += position_weight * 0.5;
        }
        
        // Add bigram features
        for i in 0..words.len().saturating_sub(1) {
            let bigram = format!("{} {}", words[i], words[i + 1]);
            let mut hasher = DefaultHasher::new();
            bigram.hash(&mut hasher);
            let hash = hasher.finish();
            
            let idx = (hash % self.embedding_dimension as u64) as usize;
            embedding[idx] += 0.8;
        }
        
        // Normalize the embedding vector
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in embedding.iter_mut() {
                *val /= magnitude;
            }
        }
        
        Ok(embedding)
    }
    
    /// Index a document chunk into the vector store
    pub fn index_document(&mut self, chunk: &DocumentChunk) -> Result<()> {
        let embedding = self.generate_embeddings(&chunk.content)?;
        
        let mut indexed_chunk = chunk.clone();
        indexed_chunk.embedding = embedding;
        
        // Remove existing document with same ID if it exists
        self.documents.retain(|doc| doc.id != chunk.id);
        
        // Add the new document
        self.documents.push(indexed_chunk);
        
        // Save to file
        self.save_to_file()?;
        
        println!("📄 Indexed chunk ({} chars)", chunk.content.len());
        Ok(())
    }
    
    /// Search for similar documents based on query
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<DocumentChunk>> {
        let query_embedding = self.generate_embeddings(query)?;
        
        let mut scored_docs: Vec<(f32, &DocumentChunk)> = self.documents
            .iter()
            .map(|doc| {
                let similarity = cosine_similarity(&query_embedding, &doc.embedding);
                (similarity, doc)
            })
            .collect();
        
        // Sort by similarity (highest first)
        scored_docs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        
        // Return top results
        let results: Vec<DocumentChunk> = scored_docs
            .into_iter()
            .take(limit)
            .map(|(_, doc)| doc.clone())
            .collect();
        
        Ok(results)
    }
    
    /// Parse HTML content and create document chunks
    pub fn parse_html_to_chunks(&self, html_content: &str, source_url: &str) -> Result<Vec<DocumentChunk>> {
        let document = Html::parse_document(html_content);
        let mut chunks = Vec::new();
        
        // Extract text from various HTML elements
        let selectors = [
            "h1", "h2", "h3", "h4", "h5", "h6",
            "p", "div", "section", "article",
            "li", "td", "th", "blockquote"
        ];
        
        for selector_str in &selectors {
            if let Ok(selector) = Selector::parse(selector_str) {
                for element in document.select(&selector) {
                    let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                    
                    if text.len() > 50 { // Only include substantial text chunks
                        let url_hash = format!("{:x}", md5::compute(source_url.as_bytes()));
                        let text_hash = format!("{:x}", md5::compute(text.as_bytes()));
                        let chunk_id = format!("{}-{}", url_hash, text_hash);
                        
                        let mut metadata = HashMap::new();
                        metadata.insert("element_type".to_string(), selector_str.to_string());
                        metadata.insert("url".to_string(), source_url.to_string());
                        
                        chunks.push(DocumentChunk {
                            id: chunk_id,
                            content: text,
                            source: source_url.to_string(),
                            metadata,
                            embedding: Vec::new(), // Will be filled during indexing
                        });
                    }
                }
            }
        }
        
        Ok(chunks)
    }
    
    /// Index a webpage by URL
    pub async fn index_webpage(&mut self, url: &str) -> Result<usize> {
        println!("🌐 Fetching webpage: {}", url);
        
        let response = reqwest::get(url).await?;
        let html_content = response.text().await?;
        
        let chunks = self.parse_html_to_chunks(&html_content, url)?;
        let chunk_count = chunks.len();
        
        for chunk in chunks {
            self.index_document(&chunk)?;
        }
        
        println!("✅ Indexed {} chunks from {}", chunk_count, url);
        Ok(chunk_count)
    }
    
    /// Get collection info
    pub fn get_collection_info(&self) -> Result<()> {
        println!("📊 Local Vector Store Info:");
        println!("   Documents count: {}", self.documents.len());
        println!("   Embedding dimension: {}", self.embedding_dimension);
        println!("   Data file: {}", self.data_file);
        Ok(())
    }
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }
    
    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    
    #[test]
    fn test_local_vector_store_creation() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = LocalVectorStore::new(temp_file.path().to_str().unwrap()).unwrap();
        assert_eq!(store.embedding_dimension, 384);
        assert_eq!(store.documents.len(), 0);
    }
    
    #[test]
    fn test_html_parsing() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = LocalVectorStore::new(temp_file.path().to_str().unwrap()).unwrap();
        
        let html = r#"
            <html>
                <body>
                    <h1>Test Title</h1>
                    <p>This is a test paragraph with enough content to be indexed.</p>
                    <div>Another content block that should be captured.</div>
                </body>
            </html>
        "#;
        
        let chunks = store.parse_html_to_chunks(html, "http://test.com").unwrap();
        assert!(chunks.len() > 0);
        
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
            assert_eq!(chunk.source, "http://test.com");
        }
    }
    
    #[test]
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let vec3 = vec![0.0, 1.0, 0.0];
        
        assert!((cosine_similarity(&vec1, &vec2) - 1.0).abs() < 0.001);
        assert!((cosine_similarity(&vec1, &vec3) - 0.0).abs() < 0.001);
    }
}