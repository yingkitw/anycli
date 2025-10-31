# TODO - AnyCLI (Cloud Universal CLI)

## Completed ✅

- [x] Upgrade to Rust edition 2024
- [x] Update all crate dependencies to latest versions
- [x] Create workspace structure with multiple crates
- [x] Define core traits (LLMProvider, RAGEngine, VectorStore, DocumentIndexer)
- [x] Implement WatsonX client with LLMProvider trait
- [x] Implement LocalVectorStore with VectorStore trait
- [x] Implement LocalDocumentIndexer with DocumentIndexer trait
- [x] Implement LocalRAGEngine with RAGEngine trait
- [x] Create CLI utilities (CommandTranslator, CommandLearningEngine, QualityAnalyzer)
- [x] Add insta for snapshot testing
- [x] Ensure cargo build success
- [x] Update documentation (README.md, ARCHITECTURE.md, TODO.md)
- [x] Rename program to AnyCLI (Cloud Universal CLI)
- [x] Migrate all capabilities from src/ to crates/
- [x] Delete legacy folder after migration
- [x] Simplify main.rs to only orchestration logic
- [x] Add multi-cloud provider abstraction (AWS, GCP, Azure, IBM Cloud, VMware)
- [x] Implement cloud provider detection from queries
- [x] Add CLI flags for provider selection
- [x] Create separate crates for each cloud provider
- [x] Implement CloudProvider trait for all 5 providers
- [x] Add provider-specific RAG context and command validation
- [x] Implement AI-powered error recovery with WatsonX suggestions
- [x] Merge features from cac project (JSON repair for AWS CLI output)
- [x] Add anyrepair dependency for JSON repair functionality

## In Progress 🚧

- [ ] Implement provider-specific RAG knowledge bases
- [ ] Add AWS CLI command translation
- [ ] Add GCP CLI command translation
- [ ] Add Azure CLI command translation
- [ ] Add VMware vSphere CLI command translation
- [ ] Fix remaining compiler warnings
- [x] Add comprehensive unit tests for all crates (51+ tests)
- [x] Improve test coverage for cloud providers
- [x] Add tests for provider configurations
- [ ] Add integration tests
- [x] Create snapshot tests with insta

## High Priority 🔴

### Testing
- [ ] Add unit tests for WatsonxClient
- [ ] Add unit tests for LocalVectorStore
- [ ] Add unit tests for LocalDocumentIndexer
- [ ] Add unit tests for LocalRAGEngine
- [ ] Add unit tests for CommandTranslator
- [ ] Add integration tests for end-to-end workflows
- [ ] Add snapshot tests for command generation

### Documentation
- [ ] Add API documentation for all public types
- [ ] Add usage examples in crate READMEs
- [ ] Create developer guide
- [ ] Add contribution guidelines

### Code Quality
- [ ] Fix all clippy warnings
- [ ] Add CI/CD pipeline
- [ ] Add code coverage reporting
- [ ] Add benchmarks for performance-critical paths

## Medium Priority 🟡

### Multi-Cloud Features
- [ ] Implement CloudProvider trait for each provider
- [ ] Add provider-specific command validation
- [ ] Create separate RAG knowledge bases per cloud
- [ ] Add CLI installation detection per provider
- [ ] Add authentication status check per provider
- [ ] Implement cross-cloud command translation
- [ ] Add provider-specific error handling

### Features
- [ ] Implement Qdrant vector store integration
- [ ] Implement web document scraper for cloud docs
- [ ] Add embedding generation for better RAG
- [ ] Add command caching per provider
- [ ] Add telemetry and metrics
- [ ] Add configuration file support (beyond .env)
- [ ] Add provider preference persistence

### Improvements
- [ ] Improve error messages
- [ ] Add progress indicators for long operations
- [ ] Add command history persistence
- [ ] Improve quality scoring algorithm
- [ ] Add more IBM Cloud CLI knowledge to RAG

### User Experience
- [ ] Add command suggestions
- [ ] Add auto-completion
- [ ] Add command preview before execution
- [ ] Add better error recovery
- [ ] Add interactive tutorials

## Low Priority 🟢

### Additional LLM Providers
- [ ] Add OpenAI provider
- [ ] Add Anthropic provider
- [ ] Add local LLM support (llama.cpp, etc.)

### Additional Vector Stores
- [ ] Add Pinecone integration
- [ ] Add Weaviate integration
- [ ] Add Milvus integration

### Additional CLI Support
- [ ] Add Kubernetes CLI support (kubectl)
- [ ] Add Terraform CLI support
- [ ] Add Docker CLI support

### Platform Support
- [ ] Test on Windows
- [ ] Test on Linux
- [ ] Create installation packages (Homebrew, apt, etc.)
- [ ] Create Docker image

### Advanced Features
- [ ] Add multi-turn conversations
- [ ] Add context-aware suggestions
- [ ] Add command composition
- [ ] Add batch command execution
- [ ] Add command scheduling

## Technical Debt 💳

- [ ] Clean up unused imports
- [ ] Consolidate error handling patterns
- [ ] Improve async/await usage patterns
- [ ] Fix non-deterministic snapshot test ordering (HashMap iteration order)

## Notes 📝

### Program Name
- **AnyCLI**: Cloud Universal CLI
- Renamed from "IBM Cloud CLI AI (icx)" to support universal cloud CLI operations
- Binary name changed from `icx` to `anycli`
- Package name changed from `ibmcloud-cli-ai` to `anycli`

### Multi-Cloud Support
- **Supported Providers**: IBM Cloud, AWS, GCP, Azure, VMware vSphere
- **Provider Detection**: Automatic detection from query keywords
- **CLI Flags**: `--provider` to specify provider, `--list-providers` to see all
- **Default Provider**: IBM Cloud (configurable)
- **VMware CLI**: Uses `govc` (vSphere CLI) for VMware operations
- **Provider Crates**: Each provider has its own crate implementing `CloudProvider` trait
  - `anycli-ibmcloud`: IBM Cloud provider
  - `anycli-aws`: AWS provider
  - `anycli-gcp`: GCP provider
  - `anycli-azure`: Azure provider
  - `anycli-vmware`: VMware vSphere provider
- **Features**: CLI detection, authentication checking, command validation, RAG context

### Migration Strategy
1. Keep old implementation in src/ for reference
2. New modular implementation in crates/
3. Gradually migrate main.rs to use new crates
4. Remove old files once migration is complete

### Testing Strategy
- Unit tests in each crate
- Integration tests in main binary
- Snapshot tests for command generation
- Property-based tests for critical logic

### Release Checklist
- [ ] All tests passing
- [ ] Documentation complete
- [ ] No compiler warnings
- [ ] Benchmarks run
- [ ] CHANGELOG updated
- [ ] Version bumped
