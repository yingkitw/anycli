# IBM Cloud CLI AI

An AI-powered assistant for the IBM Cloud CLI that translates natural language queries into IBM Cloud commands.

## Features

- **Natural Language Processing**: Convert plain English requests into IBM Cloud CLI commands
- **Interactive Chat Mode**: Engage in a conversation-like interface with the CLI
- **Command Editing**: Review and modify translated commands before execution
- **Command Execution**: Run the commands directly from the interface

## Installation

1. Clone this repository
2. Create a `.env` file with your WatsonX API credentials:
   ```
   WATSONX_API_KEY=your_api_key
   WATSONX_PROJECT_ID=your_project_id
   ```
3. Build the project:
   ```
   cargo build
   ```

## Usage

### Ask a single question

```
cargo run -- ask "list all my code engine instances"
```

### Start interactive chat mode

```
cargo run -- chat
```

In chat mode:
- Type your query in natural language
- The AI will translate it to an IBM Cloud command
- Edit the command if needed or press Enter to execute
- Type `exit` or `quit` to end the session

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
