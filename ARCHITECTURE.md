# Architecture

## Overview

IBM Cloud CLI AI is a modular, trait-based Rust application that translates natural language queries into IBM Cloud CLI commands using WatsonX AI and RAG (Retrieval-Augmented Generation).

## Rust Edition

- **Edition**: 2024
- **Compiler**: Latest stable Rust toolchain

## Workspace Structure

The project is organized as a Cargo workspace with multiple crates:

```
ibmcloud-cli-ai/
├── crates/
│   ├── core/           # Core traits and types
│   ├── watsonx/        # WatsonX AI integration
│   ├── rag/            # RAG engine, vector stores, document indexers
│   └── cli/            # CLI interface and utilities
├── src/                # Main binary
└── Cargo.toml          # Workspace configuration
```

## Crate Descriptions

### `ibmcloud-cli-ai-core`

The core crate defines fundamental traits and types used across the system:

- **Traits**:
  - `LLMProvider`: Interface for Large Language Model providers
  - `RAGEngine`: Interface for Retrieval-Augmented Generation engines
  - `VectorStore`: Interface for vector database operations
  - `DocumentIndexer`: Interface for document indexing

- **Types**:
  - `Error` and `Result`: Custom error handling
  - `GenerationConfig`, `GenerationResult`: LLM generation types
  - `RAGQuery`, `RAGResult`: RAG query types
  - `VectorDocument`, `SearchConfig`: Vector store types
  - `Document`, `IndexingConfig`: Document indexer types

### `ibmcloud-cli-ai-watsonx`

WatsonX AI integration crate:

- **`WatsonxClient`**: Implements `LLMProvider` trait
- **`WatsonxConfig`**: Configuration for WatsonX API
- **Features**:
  - OAuth2 authentication with IBM Cloud IAM
  - Streaming text generation
  - Retry logic with quality assessment
  - Feedback-based prompt enhancement

### `ibmcloud-cli-ai-rag`

RAG engine and supporting components:

- **`LocalRAGEngine`**: Implements `RAGEngine` trait
- **`LocalVectorStore`**: In-memory vector store implementing `VectorStore`
- **`QdrantVectorStore`**: Qdrant integration (future)
- **`LocalDocumentIndexer`**: Document indexing with chunking
- **`WebDocumentIndexer`**: Web scraping and indexing (future)

### `ibmcloud-cli-ai-cli`

CLI interface and utilities:

- **`CommandTranslator`**: Translates natural language to CLI commands
- **`CommandLearningEngine`**: Learns from user corrections
- **`QualityAnalyzer`**: Assesses command quality
- **`UI utilities`**: Banner display, input handling

## Design Principles

### 1. Trait-Based Architecture

All major components are defined as traits, making the system:
- **Testable**: Easy to create mock implementations
- **Extensible**: New providers can be added without changing existing code
- **Modular**: Clear separation of concerns

### 2. Capability-Facing Interfaces

Traits represent capabilities rather than implementations:
- `LLMProvider` abstracts any LLM (WatsonX, OpenAI, etc.)
- `VectorStore` abstracts any vector database (local, Qdrant, Pinecone, etc.)
- `RAGEngine` abstracts any RAG implementation

### 3. Separation of Concerns

Each crate has a single, well-defined responsibility:
- **core**: Defines interfaces
- **watsonx**: Implements LLM provider
- **rag**: Implements RAG components
- **cli**: Implements user interface

### 4. Test-Friendly Design

- Traits enable easy mocking
- `insta` for snapshot testing
- Each crate has its own test suite
- Integration tests in the main binary

## Data Flow

```
User Input (Natural Language)
    ↓
CLI Interface
    ↓
CommandTranslator
    ↓
RAGEngine (optional context enhancement)
    ↓
LLMProvider (WatsonX)
    ↓
Generated Command
    ↓
Quality Analyzer
    ↓
User Confirmation
    ↓
Command Execution
    ↓
Learning Engine (if correction needed)
```

## Key Technologies

- **Rust 2024**: Latest Rust edition
- **Tokio**: Async runtime
- **Reqwest**: HTTP client for API calls
- **Clap**: CLI argument parsing
- **Serde**: Serialization/deserialization
- **Crossterm**: Terminal UI
- **Colored**: Terminal colors
- **Insta**: Snapshot testing
- **Pulldown-cmark**: Markdown parsing

## Configuration

Configuration is loaded from environment variables (`.env` file):

```
WATSONX_API_KEY=<your-api-key>
WATSONX_PROJECT_ID=<your-project-id>
IAM_IBM_CLOUD_URL=iam.cloud.ibm.com (optional)
WATSONX_API_URL=https://us-south.ml.cloud.ibm.com (optional)
```

## Error Handling

Custom error types defined in `core::Error`:
- `LLMProvider`: LLM-related errors
- `RAGEngine`: RAG-related errors
- `VectorStore`: Vector store errors
- `DocumentIndexer`: Indexing errors
- `Configuration`: Configuration errors
- `Authentication`: Auth errors
- `Network`: Network errors
- `Timeout`: Timeout errors

## Future Enhancements

1. **Additional LLM Providers**: OpenAI, Anthropic, etc.
2. **Qdrant Integration**: Full Qdrant vector store support
3. **Web Scraping**: Complete web document indexer
4. **Embeddings**: Add embedding generation for better RAG
5. **Caching**: Cache frequently used commands
6. **Telemetry**: Add observability and metrics
