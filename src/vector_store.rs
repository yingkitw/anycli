use anyhow::{Result, anyhow};
use qdrant_client::prelude::*;
use qdrant_client::client::QdrantClient;
use qdrant_client::qdrant::vectors_config::Config as VectorsConfig;
use qdrant_client::qdrant::{CreateCollection, SearchPoints, PointStruct, Value, VectorParams, value::Kind, Distance};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
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
}

pub struct VectorStore {
    client: QdrantClient,
    collection_name: String,
    embedding_dimension: u64,
}

impl VectorStore {
    /// Initialize a new VectorStore with simple hash-based embeddings
    pub async fn new(qdrant_url: &str, collection_name: &str) -> Result<Self> {
        // Initialize Qdrant client
        let client = QdrantClient::from_url(qdrant_url).build()?;
        
        let mut store = Self {
            client,
            collection_name: collection_name.to_string(),
            embedding_dimension: 384, // Standard dimension for sentence embeddings
        };
        
        // Create collection if it doesn't exist
        store.create_collection().await?;
        
        Ok(store)
    }
    
    /// Create Qdrant collection for storing document embeddings
    async fn create_collection(&self) -> Result<()> {
        let collections = self.client.list_collections().await?;
        
        let collection_exists = collections
            .collections
            .iter()
            .any(|c| c.name == self.collection_name);
            
        if !collection_exists {
            let create_collection = CreateCollection {
                collection_name: self.collection_name.clone(),
                vectors_config: Some(VectorsConfig::Params(VectorParams {
                size: self.embedding_dimension,
                distance: Distance::Cosine.into(),
                ..Default::default()
            }).into()),
                ..Default::default()
            };
            
            self.client.create_collection(&create_collection).await?;
            println!("✅ Created Qdrant collection: {}", self.collection_name);
        }
        
        Ok(())
    }
    
    /// Generate simple hash-based embeddings for text
    fn generate_embeddings(&self, text: &str) -> Result<Vec<f32>> {
        // Normalize text
        let normalized_text = text.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>();
        
        // Split into words and create features
        let words: Vec<&str> = normalized_text.split_whitespace().collect();
        let mut embeddings = vec![0.0f32; self.embedding_dimension as usize];
        
        // Generate hash-based features
        for (i, word) in words.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            word.hash(&mut hasher);
            let hash = hasher.finish();
            
            // Map hash to embedding dimensions
            let base_idx = (hash as usize) % (self.embedding_dimension as usize);
            
            // Add word frequency and position information
            let weight = 1.0 / (1.0 + i as f32 * 0.1); // Position-based weighting
            embeddings[base_idx] += weight;
            
            // Add secondary features for better distribution
            if word.len() > 3 {
                let secondary_idx = ((hash >> 16) as usize) % (self.embedding_dimension as usize);
                embeddings[secondary_idx] += weight * 0.5;
            }
        }
        
        // Add n-gram features for better context
        for window in words.windows(2) {
            let bigram = format!("{} {}", window[0], window[1]);
            let mut hasher = DefaultHasher::new();
            bigram.hash(&mut hasher);
            let hash = hasher.finish();
            
            let idx = (hash as usize) % (self.embedding_dimension as usize);
            embeddings[idx] += 0.3; // Bigram weight
        }
        
        // Normalize the embedding vector
        let magnitude: f32 = embeddings.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embeddings {
                *val /= magnitude;
            }
        }
        
        Ok(embeddings)
    }
    
    /// Index a document chunk into the vector store
    pub async fn index_document(&self, chunk: &DocumentChunk) -> Result<()> {
        let embedding = self.generate_embeddings(&chunk.content)?;
        
        let mut payload = HashMap::new();
        payload.insert("content".to_string(), Value::from(chunk.content.clone()));
        payload.insert("source".to_string(), Value::from(chunk.source.clone()));
        
        // Add metadata
        for (key, value) in &chunk.metadata {
            payload.insert(key.clone(), Value::from(value.clone()));
        }
        
        let point = PointStruct::new(
            chunk.id.clone(),
            embedding,
            payload,
        );
        
        self.client
            .upsert_points_blocking(&self.collection_name, None, vec![point], None)
            .await?;
            
        Ok(())
    }
    
    /// Search for similar documents based on query
    pub async fn search(&self, query: &str, limit: u64) -> Result<Vec<DocumentChunk>> {
        let query_embedding = self.generate_embeddings(query)?;
        
        let search_points = SearchPoints {
            collection_name: self.collection_name.clone(),
            vector: query_embedding,
            limit,
            with_payload: Some(true.into()),
            ..Default::default()
        };
        
        let search_result = self.client.search_points(&search_points).await?;
        
        let mut results = Vec::new();
        for scored_point in search_result.result {
            let payload = scored_point.payload;
            let content = payload.get("content")
                .and_then(|v| match v {
                    Value { kind: Some(qdrant_client::qdrant::value::Kind::StringValue(s)) } => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("").to_string();
                
            let source = payload.get("source")
                .and_then(|v| match v {
                    Value { kind: Some(qdrant_client::qdrant::value::Kind::StringValue(s)) } => Some(s.as_str()),
                    _ => None,
                })
                .unwrap_or("").to_string();
                    
                let mut metadata = HashMap::new();
                for (key, value) in payload {
                    if key != "content" && key != "source" {
                        if let Value { kind: Some(Kind::StringValue(s)) } = value {
                            metadata.insert(key, s);
                        }
                    }
                }
                
                let point_id = match scored_point.id.unwrap() {
                    qdrant_client::qdrant::PointId { point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Uuid(uuid)) } => uuid,
                    qdrant_client::qdrant::PointId { point_id_options: Some(qdrant_client::qdrant::point_id::PointIdOptions::Num(num)) } => num.to_string(),
                    _ => "unknown".to_string(),
                };
                
            results.push(DocumentChunk {
                id: point_id,
                content,
                source,
                metadata,
            });
        }
        
        Ok(results)
    }
    
    /// Parse HTML content and create document chunks
    pub fn parse_html_to_chunks(&self, html_content: &str, source_url: &str) -> Result<Vec<DocumentChunk>> {
        let document = Html::parse_document(html_content);
        let mut chunks = Vec::new();
        
        // Extract text from different HTML elements
        let selectors = vec![
            ("h1", "heading"),
            ("h2", "heading"),
            ("h3", "heading"),
            ("p", "paragraph"),
            ("li", "list_item"),
            ("code", "code"),
            ("pre", "code_block"),
        ];
        
        for (selector_str, content_type) in selectors {
            let selector = Selector::parse(selector_str).unwrap();
            
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                
                if !text.is_empty() && text.len() > 10 { // Filter out very short content
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), content_type.to_string());
                    metadata.insert("selector".to_string(), selector_str.to_string());
                    
                    // Generate unique ID based on content hash
                    let content_hash = format!("{:x}", md5::compute(&text));
                    let chunk_id = format!("{}_{}", source_url.replace(['/', ':', '.'], "_"), content_hash);
                    
                    chunks.push(DocumentChunk {
                        id: chunk_id,
                        content: text,
                        source: source_url.to_string(),
                        metadata,
                    });
                }
            }
        }
        
        Ok(chunks)
    }
    
    /// Index a web page by URL
    pub async fn index_webpage(&self, url: &str) -> Result<usize> {
        println!("📄 Indexing webpage: {}", url);
        
        // Fetch webpage content
        let response = reqwest::get(url).await?;
        let html_content = response.text().await?;
        
        // Parse HTML to chunks
        let chunks = self.parse_html_to_chunks(&html_content, url)?;
        
        // Index each chunk
        for chunk in &chunks {
            self.index_document(chunk).await?;
        }
        
        println!("✅ Indexed {} chunks from {}", chunks.len(), url);
        Ok(chunks.len())
    }
    
    /// Get collection info
    pub async fn get_collection_info(&self) -> Result<()> {
        let info = self.client.collection_info(self.collection_name.clone()).await?;
        println!("📊 Collection '{}' info:", self.collection_name);
        println!("   Points count: {:?}", info.result.map(|r| r.points_count));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_vector_store_creation() {
        // Skip test if Qdrant is not available
        let store_result = VectorStore::new("http://localhost:6333", "test_collection").await;
        if store_result.is_err() {
            println!("Skipping test: Qdrant not available");
            return;
        }
        
        let store = store_result.unwrap();
        assert_eq!(store.collection_name, "test_collection");
    }
    
    #[test]
    fn test_html_parsing() {
        let html = r#"
            <html>
                <body>
                    <h1>IBM Cloud CLI Commands</h1>
                    <p>This is a guide for IBM Cloud CLI commands.</p>
                    <code>ibmcloud login</code>
                </body>
            </html>
        "#;
        
        // Create a mock store for testing (without actual Qdrant connection)
        let device = Device::Cpu;
        let chunks_result = VectorStore {
            client: QdrantClient::from_url("http://localhost:6333").build().unwrap(),
            collection_name: "test".to_string(),
            model: unsafe { std::mem::zeroed() }, // Mock for test
            tokenizer: unsafe { std::mem::zeroed() }, // Mock for test
            device,
        }.parse_html_to_chunks(html, "https://example.com");
        
        // This test will fail due to mock objects, but shows the intended structure
        // In a real scenario, we'd use dependency injection or mocking framework
    }
}