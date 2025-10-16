# IBM Cloud CLI AI (icx)

An AI-powered assistant for the IBM Cloud CLI that translates natural language queries into IBM Cloud commands using WatsonX AI.

## Architecture

Built with **Rust 2024** and a modular, trait-based architecture for maximum testability and extensibility.

### Workspace Structure

```
ibmcloud-cli-ai/
├── crates/
│   ├── core/           # Core traits and types
│   ├── watsonx/        # WatsonX AI integration
│   ├── rag/            # RAG engine and vector stores
│   └── cli/            # CLI interface utilities
└── src/                # Main binary
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed architecture documentation.

## Features

### Core Capabilities
- **Natural Language Processing**: Convert plain English requests into IBM Cloud CLI commands
- **Interactive Chat Mode**: Engage in a conversation-like interface with the CLI
- **Enhanced Command Editing**: Review and modify translated commands before execution with Esc to cancel
- **Command History Navigation**: Use ↑/↓ arrow keys to navigate through previous commands
- **Command Execution**: Run the commands directly from the interface
- **Interactive Command Support**: Automatically handles interactive commands like SSO login
- **Login Status Check**: Automatically checks if you're logged into IBM Cloud before executing commands

### AI-Powered Features
- **🧠 Intelligent Learning System**: AI-powered command learning that captures user corrections and improves suggestions over time
- **🔌 Smart Plugin Error Handling**: Detects missing plugin errors and provides specific guidance for installation and alternatives
- **📚 Local RAG Engine**: Enhanced knowledge base with IBM Cloud CLI documentation for better command suggestions
- **💡 Interactive Error Recovery**: Intelligent error pattern recognition with contextual suggestions and learning capabilities
- **🔧 Robust WatsonX Integration**: Improved API response handling with enhanced prompt engineering and error recovery
- **⚡ Pipeline Input Support**: Seamlessly handles both interactive and pipeline input modes for automation workflows

### Technical Features
- **Trait-Based Architecture**: Modular design with clear separation of concerns
- **Test-Friendly**: Easy to mock and test with comprehensive test coverage
- **Extensible**: Add new LLM providers or vector stores without changing existing code
- **Type-Safe**: Leverages Rust's type system for reliability

## Installation

1. Clone this repository
2. Create a `.env` file with your WatsonX API credentials:
   ```
   WATSONX_API_KEY=your_api_key
   WATSONX_PROJECT_ID=your_project_id
   ```
3. Build and install the CLI:
   ```
   cargo build --release
   cargo install --path .
   ```

## Usage

Simply run the CLI to start the interactive chat mode:

```
icx
```

In chat mode:
- Type your query in natural language
- The AI will translate it to an IBM Cloud command
- Edit the command if needed or press Enter to execute, Esc to cancel
- Use ↑/↓ arrow keys to navigate through command history
- Type `exec <command>` to execute a command directly
- Type `exit` or `quit` to end the session

### Key Features in Action

**Command History Navigation:**
- Press ↑ to go back through previous commands
- Press ↓ to go forward through command history
- Edit any recalled command before execution

**Enhanced Command Editing:**
- Press Enter to execute the suggested command
- Press Esc to cancel and return to chat
- Use Backspace to edit the command
- Type new characters to modify the command

### Examples

```
ibmcloud-ai> list my code engine applications
ibmcloud-ai> show me all my kubernetes clusters
ibmcloud-ai> create a new resource group called my-project
ibmcloud-ai> exec ibmcloud target --cf
```

## 🧠 Learning System

The AI assistant now includes an intelligent learning system that improves over time:

### Command Learning
- **Error Correction Learning**: When commands fail, you can provide the correct command and the system learns from it
- **Pattern Recognition**: The system identifies common error patterns and suggests fixes
- **Contextual Suggestions**: Based on previous corrections, the system provides better command suggestions

### Enhanced Error Handling
- **Plugin Detection**: Automatically detects when commands fail due to missing plugins
- **Installation Guidance**: Provides specific instructions for installing required plugins
- **Alternative Suggestions**: Offers alternative commands that don't require plugins

### Example Learning Interaction
```
ibmcloud-ai> list my databases
❌ Command failed: 'dbs' is not a registered command. Check 'ibmcloud plugin list' for available plug-ins.

🔌 This appears to be a missing plugin. You may need to:
  • Install the required plugin with 'ibmcloud plugin install <plugin-name>'
  • Check available plugins with 'ibmcloud plugin repo-plugins'
  • Or use an alternative command that doesn't require plugins

📝 If you know the correct command, I can learn from this for future requests.
Enter the correct command (or press Enter to skip): ibmcloud resource service-instances --service-name databases-for-postgresql

✅ Thanks! I've learned that 'list my databases' should be 'ibmcloud resource service-instances --service-name databases-for-postgresql'
```

## 🔧 Recent Technical Improvements

### WatsonX API Integration Enhancements
Recent updates have significantly improved the reliability and accuracy of the WatsonX AI integration:

- **Enhanced Response Parsing**: Improved handling of Server-Sent Events (SSE) format responses from WatsonX API
- **Optimized Generation Parameters**: Adjusted `min_new_tokens` and refined stop sequences for better command generation
- **Robust Error Handling**: Better detection and recovery from API response issues
- **Improved Prompt Engineering**: Enhanced prompt structure with explicit query inclusion for more accurate translations
- **Pipeline Input Support**: Fixed infinite loop issues when processing piped input, enabling automation workflows

### Quality Improvements
- **Response Cleaning**: Automatic removal of unwanted prefixes ("Answer:") and suffixes ("Query:") from generated commands
- **First-Line Extraction**: Ensures only the actual command is returned, filtering out extraneous text
- **Empty Response Prevention**: Enhanced validation to prevent empty or malformed command generation

### Usage Examples with Pipeline Input
```bash
# Pipeline input for automation
echo "list all resource groups" | icx
echo "show me all wml instances" | icx

# Interactive mode
icx
```

## IBM Cloud CLI References

- [IBM Cloud CLI](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_cli)
- [Resource Management](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_commands_resource)
- [IAM Commands](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_commands_iam)
- [Kubernetes Service](https://cloud.ibm.com/docs/cli?topic=cli-kubernetes-service-cli)
- [Catalog Management](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_catalog)

## Development

### Prerequisites

- Rust 1.80+ (Rust 2024 edition)
- IBM Cloud CLI installed
- WatsonX API credentials

### Building

```bash
# Build all workspace crates
cargo build --workspace

# Build in release mode
cargo build --release --workspace

# Run tests
cargo test --workspace

# Check for issues
cargo check --workspace
```

### Project Structure

- **`crates/core`**: Core traits and types (`LLMProvider`, `RAGEngine`, `VectorStore`, `DocumentIndexer`)
- **`crates/watsonx`**: WatsonX AI client implementation
- **`crates/rag`**: RAG engine, vector stores, and document indexers
- **`crates/cli`**: CLI utilities (translator, learning engine, quality analyzer)

### Adding a New LLM Provider

1. Implement the `LLMProvider` trait from `ibmcloud-cli-ai-core`
2. Add your implementation to a new crate or the `watsonx` crate
3. Update the main binary to use your provider

Example:
```rust
use ibmcloud_cli_ai_core::{LLMProvider, GenerationConfig, GenerationResult};
use async_trait::async_trait;

pub struct MyLLMProvider {
    // your fields
}

#[async_trait]
impl LLMProvider for MyLLMProvider {
    async fn connect(&mut self) -> Result<()> {
        // implementation
    }
    
    async fn generate(&self, prompt: &str) -> Result<GenerationResult> {
        // implementation
    }
    
    // ... other trait methods
}
```

### Testing

The project uses `insta` for snapshot testing:

```bash
# Run tests
cargo test

# Review snapshots
cargo insta review

# Accept all snapshots
cargo insta accept
```

## Contributing

See [TODO.md](TODO.md) for planned features and improvements.

## Powered By

- Rust 2024
- WatsonX AI (IBM Granite models)
- IBM Cloud CLI
- Tokio (async runtime)
- Clap (CLI parsing)
- Insta (snapshot testing)
