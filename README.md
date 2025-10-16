# CUC (Cloud Universal CLI)

An AI-powered universal CLI assistant that translates natural language queries into cloud commands using WatsonX AI. Supports multiple cloud providers including IBM Cloud, AWS, GCP, Azure, and VMware vSphere.

## Architecture

Built with **Rust 2024** and a modular, trait-based architecture for maximum testability and extensibility.

### Workspace Structure

```
cuc/
├── crates/
│   ├── cuc-core/       # Core traits and types
│   ├── cuc-watsonx/    # WatsonX AI integration
│   ├── cuc-rag/        # RAG engine and vector stores
│   ├── cuc-cli/        # CLI interface utilities
│   ├── cuc-ibmcloud/   # IBM Cloud provider
│   ├── cuc-aws/        # AWS provider
│   ├── cuc-gcp/        # GCP provider
│   ├── cuc-azure/      # Azure provider
│   └── cuc-vmware/     # VMware vSphere provider
└── src/                # Main binary
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for detailed architecture documentation.

## Features

### Core Capabilities
- **Multi-Cloud Support**: Unified interface for IBM Cloud, AWS, GCP, Azure, and VMware CLIs
- **Natural Language Processing**: Convert plain English requests into cloud CLI commands
- **Interactive Chat Mode**: Engage in a conversation-like interface with the CLI
- **Enhanced Command Editing**: Review and modify translated commands before execution with Esc to cancel
- **Command History Navigation**: Use ↑/↓ arrow keys to navigate through previous commands
- **Command Execution**: Run the commands directly from the interface
- **Interactive Command Support**: Automatically handles interactive commands like SSO login
- **Login Status Check**: Automatically checks if you're logged in before executing commands

### AI-Powered Features
- **🧠 Intelligent Learning System**: AI-powered command learning that captures user corrections and improves suggestions over time
- **🔌 Smart Plugin Error Handling**: Detects missing plugin errors and provides specific guidance for installation and alternatives
- **📚 Local RAG Engine**: Enhanced knowledge base with cloud CLI documentation for better command suggestions
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
cuc
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

**IBM Cloud:**
```
cuc> list my code engine applications
cuc> show me all my kubernetes clusters
cuc> create a new resource group called my-project
```

**AWS:**
```
cuc> list all my ec2 instances
cuc> show s3 buckets in us-east-1
cuc> create a lambda function
```

**GCP:**
```
cuc> list compute instances
cuc> show all cloud storage buckets
cuc> create a new gke cluster
```

**Azure:**
```
cuc> list virtual machines
cuc> show storage accounts
cuc> create a resource group
```

**VMware vSphere:**
```
cuc> list all vms
cuc> show esxi hosts
cuc> power on virtual machine
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
cuc> list my databases
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
echo "list all resource groups" | cuc
echo "show me all wml instances" | cuc

# Interactive mode
cuc
```

## Cloud CLI References

### IBM Cloud
- [IBM Cloud CLI](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_cli)
- [Resource Management](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_commands_resource)
- [IAM Commands](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_commands_iam)
- [Kubernetes Service](https://cloud.ibm.com/docs/cli?topic=cli-kubernetes-service-cli)

### AWS
- [AWS CLI](https://aws.amazon.com/cli/)
- [AWS CLI Command Reference](https://docs.aws.amazon.com/cli/latest/reference/)
- [EC2 Commands](https://docs.aws.amazon.com/cli/latest/reference/ec2/)
- [S3 Commands](https://docs.aws.amazon.com/cli/latest/reference/s3/)

### GCP
- [gcloud CLI](https://cloud.google.com/sdk/gcloud)
- [gcloud Reference](https://cloud.google.com/sdk/gcloud/reference)
- [Compute Commands](https://cloud.google.com/sdk/gcloud/reference/compute)
- [Storage Commands](https://cloud.google.com/sdk/gcloud/reference/storage)

### Azure
- [Azure CLI](https://docs.microsoft.com/en-us/cli/azure/)
- [Azure CLI Reference](https://docs.microsoft.com/en-us/cli/azure/reference-index)
- [VM Commands](https://docs.microsoft.com/en-us/cli/azure/vm)
- [Storage Commands](https://docs.microsoft.com/en-us/cli/azure/storage)

### VMware
- [govc (vSphere CLI)](https://github.com/vmware/govmomi/tree/master/govc)
- [VMware Cloud CLI](https://docs.vmware.com/en/VMware-Cloud-services/services/Using-VMware-Cloud-Services/GUID-3B6C0A9E-4F5E-4D8D-9F5E-0F6F5E5E5E5E.html)
- [vSphere CLI Reference](https://github.com/vmware/govmomi/blob/master/govc/USAGE.md)

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

- **`crates/cuc-core`**: Core traits and types (`LLMProvider`, `RAGEngine`, `VectorStore`, `DocumentIndexer`)
- **`crates/cuc-watsonx`**: WatsonX AI client implementation
- **`crates/cuc-rag`**: RAG engine, vector stores, and document indexers
- **`crates/cuc-cli`**: CLI utilities (translator, learning engine, quality analyzer)

### Adding a New LLM Provider

1. Implement the `LLMProvider` trait from `ibmcloud-cli-ai-core`
2. Add your implementation to a new crate or the `watsonx` crate
3. Update the main binary to use your provider

Example:
```rust
use cuc_core::{LLMProvider, GenerationConfig, GenerationResult};
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
