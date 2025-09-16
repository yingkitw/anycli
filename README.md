# IBM Cloud CLI AI (icx)

An AI-powered assistant for the IBM Cloud CLI that translates natural language queries into IBM Cloud commands.

## Features

- **Natural Language Processing**: Convert plain English requests into IBM Cloud CLI commands
- **Interactive Chat Mode**: Engage in a conversation-like interface with the CLI
- **Enhanced Command Editing**: Review and modify translated commands before execution with Esc to cancel
- **Command History Navigation**: Use ↑/↓ arrow keys to navigate through previous commands
- **Command Execution**: Run the commands directly from the interface
- **Interactive Command Support**: Automatically handles interactive commands like SSO login
- **Login Status Check**: Automatically checks if you're logged into IBM Cloud before executing commands
- **Professional Startup Banner**: Clean, informative banner displaying features and version info
- **🧠 Intelligent Learning System**: AI-powered command learning that captures user corrections and improves suggestions over time
- **🔌 Smart Plugin Error Handling**: Detects missing plugin errors and provides specific guidance for installation and alternatives
- **📚 Local RAG Engine**: Enhanced knowledge base with IBM Cloud CLI documentation for better command suggestions
- **💡 Interactive Error Recovery**: Intelligent error pattern recognition with contextual suggestions and learning capabilities

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

## IBM Cloud CLI References

- [IBM Cloud CLI](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_cli)
- [Resource Management](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_commands_resource)
- [IAM Commands](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_commands_iam)
- [Kubernetes Service](https://cloud.ibm.com/docs/cli?topic=cli-kubernetes-service-cli)
- [Catalog Management](https://cloud.ibm.com/docs/cli?topic=cli-ibmcloud_catalog)

## Powered By

- Rust
- WatsonX AI
- IBM Cloud CLI
