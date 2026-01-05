# Architecture

## Overview

AnyCLI (Cloud Universal CLI) is a Domain-Driven Design (DDD) based Rust application that translates natural language queries into cloud CLI commands using WatsonX AI and RAG (Retrieval-Augmented Generation). It provides a unified interface for multiple cloud providers including IBM Cloud, AWS, GCP, Azure, and VMware vSphere.

## Architecture Pattern

The application follows **Domain-Driven Design (DDD)** principles with clear separation of concerns across three layers:

1. **Domain Layer**: Core business logic, entities, value objects, and domain services
2. **Application Layer**: Use cases and application services that orchestrate domain logic
3. **Infrastructure Layer**: External services, repository implementations, and adapters

## Rust Edition

- **Edition**: 2024
- **Compiler**: Latest stable Rust toolchain

## Project Structure

The project follows DDD layering:

```
anycli/
├── src/
│   ├── domain/              # Domain Layer
│   │   ├── entities.rs       # Domain entities (Command, CommandLearning, CloudProvider)
│   │   ├── value_objects.rs # Value objects (QualityAnalysis, NaturalLanguageQuery)
│   │   ├── services.rs       # Domain services (traits)
│   │   └── repositories.rs  # Repository interfaces
│   ├── application/          # Application Layer
│   │   └── use_cases.rs      # Use cases (TranslateCommand, AnalyzeQuality, LearnFromCorrection)
│   ├── infrastructure/       # Infrastructure Layer
│   │   ├── adapters.rs       # External service adapters
│   │   ├── repositories.rs   # Repository implementations
│   │   └── services.rs       # Infrastructure service implementations
│   ├── core/                 # Core traits and types (legacy, being migrated to domain)
│   ├── cli/                  # CLI interface and utilities
│   ├── rag/                  # RAG engine implementations
│   ├── providers/            # Cloud provider implementations
│   ├── watsonx_adapter.rs    # WatsonX LLM adapter
│   └── main.rs               # Application entry point
└── Cargo.toml
```

### DDD Layers

#### Domain Layer (`src/domain/`)

**Entities**:
- `Command`: Represents a CLI command with quality metrics
- `CommandLearning`: Represents learned corrections from user feedback
- `CloudProvider`: Enum for supported cloud providers

**Value Objects**:
- `QualityAnalysis`: Immutable quality assessment result
- `NaturalLanguageQuery`: Immutable query representation

**Domain Services** (traits):
- `CommandQualityService`: Analyzes command quality
- `CommandTranslationService`: Translates natural language to commands
- `CommandLearningService`: Manages command learning

**Repositories** (interfaces):
- `CommandLearningRepository`: Abstract data access for learning data

#### Application Layer (`src/application/`)

**Use Cases**:
- `TranslateCommandUseCase`: Orchestrates command translation
- `AnalyzeCommandQualityUseCase`: Orchestrates quality analysis
- `LearnFromCorrectionUseCase`: Orchestrates learning from corrections

#### Infrastructure Layer (`src/infrastructure/`)

**Adapters**: Bridge between domain and external services
- WatsonX LLM adapter
- RAG engine adapters
- Cloud provider adapters

**Repository Implementations**:
- `FileCommandLearningRepository`: File-based learning data storage

**Service Implementations**:
- `QualityAnalyzerService`: Implements `CommandQualityService`
- `CommandTranslatorService`: Implements `CommandTranslationService`
- `CommandLearningServiceImpl`: Implements `CommandLearningService`

## Crate Descriptions

### `anycli-core`

The core crate defines fundamental traits and types used across the system:

- **Traits**:
  - `LLMProvider`: Interface for Large Language Model providers
  - `RAGEngine`: Interface for Retrieval-Augmented Generation engines
  - `VectorStore`: Interface for vector database operations
  - `DocumentIndexer`: Interface for document indexing
  - `CloudProvider`: Interface for cloud-specific command translation (future)

- **Types**:
  - `Error` and `Result`: Custom error handling
  - `GenerationConfig`, `GenerationResult`: LLM generation types
  - `RAGQuery`, `RAGResult`: RAG query types
  - `VectorDocument`, `SearchConfig`: Vector store types
  - `Document`, `IndexingConfig`: Document indexer types
  - `CloudProviderType`: Enum for supported cloud providers

### `anycli-rag`

RAG engine and supporting components:

- **`LocalRAGEngine`**: Implements `RAGEngine` trait
- **`LocalVectorStore`**: In-memory vector store implementing `VectorStore`
- **`QdrantVectorStore`**: Qdrant integration (future)
- **`LocalDocumentIndexer`**: Document indexing with chunking
- **`WebDocumentIndexer`**: Web scraping and indexing (future)

### `anycli-cli`

CLI interface and utilities:

- **`CommandTranslator`**: Translates natural language to CLI commands
- **`CommandLearningEngine`**: Learns from user corrections
- **`QualityAnalyzer`**: Assesses command quality
- **`UI utilities`**: Banner display, input handling

### Cloud Provider Crates

Each cloud provider has its own dedicated crate implementing the `CloudProvider` trait:

#### `anycli-ibmcloud`
- IBM Cloud CLI (`ibmcloud`) integration
- Resource management, Kubernetes, Code Engine, Cloud Foundry
- Authentication status checking
- Provider-specific RAG context

#### `anycli-aws`
- AWS CLI (`aws`) integration
- EC2, S3, Lambda, EKS operations
- AWS STS authentication checking
- Provider-specific command patterns

#### `anycli-gcp`
- Google Cloud CLI (`gcloud`) integration
- Compute Engine, Cloud Storage, GKE, Cloud Functions
- GCP auth status checking
- Provider-specific command validation

#### `anycli-azure`
- Azure CLI (`az`) integration
- Virtual Machines, Storage Accounts, AKS, Functions
- Azure account authentication checking
- Provider-specific command patterns

#### `anycli-vmware`
- VMware vSphere CLI (`govc`) integration
- VM management, ESXi hosts, vCenter operations
- vSphere authentication checking
- Provider-specific command patterns

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

### 4. Domain-Driven Design (DDD)

- **Domain Layer**: Pure business logic, no infrastructure dependencies
- **Application Layer**: Use cases orchestrate domain logic
- **Infrastructure Layer**: Implements domain interfaces, handles external concerns
- **Dependency Inversion**: Domain defines interfaces, infrastructure implements them

### 5. Test-Friendly Design

- Domain logic is pure and easily testable
- Repository interfaces enable easy mocking
- Use cases can be tested in isolation
- Regular assertions instead of snapshot testing

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
- **Pulldown-cmark**: Markdown parsing
- **async-trait**: Async trait support for domain services

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

## Multi-Cloud Architecture

### Cloud Provider Support

AnyCLI supports multiple cloud providers through a unified translation layer:

```
User Query (Natural Language)
    ↓
Cloud Provider Detection (auto or explicit)
    ↓
Provider-Specific RAG Context
    ↓
LLM Translation (with provider context)
    ↓
Provider-Specific Command
```

### Supported Cloud Providers

1. **IBM Cloud** (`ibmcloud`)
   - Resource management
   - Kubernetes clusters
   - Cloud Foundry
   - Watson services

2. **AWS** (`aws`)
   - EC2 instances
   - S3 storage
   - Lambda functions
   - EKS clusters

3. **GCP** (`gcloud`)
   - Compute Engine
   - Cloud Storage
   - GKE clusters
   - Cloud Functions

4. **Azure** (`az`)
   - Virtual Machines
   - Storage Accounts
   - AKS clusters
   - Azure Functions

5. **VMware vSphere** (`govc`)
   - Virtual Machines
   - ESXi Hosts
   - vCenter management
   - Datastores and networks

### Provider Detection

- **Automatic**: Based on installed CLIs and context keywords
- **Explicit**: User can specify provider with flags or commands
- **Default**: Configurable default provider in settings

## Migration to DDD

The codebase is being migrated to DDD architecture:
- ✅ Domain layer created with entities, value objects, and services
- ✅ Application layer with use cases
- ✅ Infrastructure layer with repository and service implementations
- 🔄 Legacy code in `core/`, `cli/`, `rag/` is being gradually migrated
- 📝 New features should use DDD patterns

## Future Enhancements

1. **Additional LLM Providers**: OpenAI, Anthropic, etc.
2. **Qdrant Integration**: Full Qdrant vector store support
3. **Web Scraping**: Complete web document indexer for cloud docs
4. **Embeddings**: Add embedding generation for better RAG
5. **Caching**: Cache frequently used commands per provider
6. **Telemetry**: Add observability and metrics
7. **Provider-Specific RAG**: Separate knowledge bases per cloud
8. **Cross-Cloud Operations**: Translate commands across providers
9. **Complete DDD Migration**: Migrate all legacy code to DDD structure
