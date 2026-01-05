# AnyCLI Deployment Guide

## Overview

AnyCLI now supports two main capabilities:

1. **Natural Language Command Translation**: Suggest cloud CLI commands from natural language
2. **Cloud Deployment**: Deploy applications to IBM Code Engine

## Features

### 1. Natural Language Command Suggestions

AnyCLI uses AI (WatsonX) with RAG (Retrieval-Augmented Generation) to translate natural language queries into cloud CLI commands.

**Usage:**
```bash
# Interactive mode (default)
anycli

# Or translate directly
anycli translate "list my resource groups"
```

**Examples:**
- `list my resource groups` → `ibmcloud resource groups`
- `show all ec2 instances` → `aws ec2 describe-instances`
- `list compute instances` → `gcloud compute instances list`
- `show my databases` → `ibmcloud resource service-instances --service-name databases`

### 2. Cloud Deployment

Deploy applications to IBM Code Engine using natural language or direct commands.

#### Natural Language Deployment

In interactive mode, simply ask to deploy:

```bash
anycli
> deploy my app to code engine
> deploy app named myapp to project myproject
> deploy to code engine
```

The system will:
1. Detect the deployment intent
2. Extract app name and project name (if provided)
3. Ask for confirmation
4. Deploy the application

#### Direct Deployment Command

```bash
anycli deploy \
  --app-name myapp \
  --project-name myproject \
  --source . \
  --region us-south \
  --resource-group Default \
  --port 8000 \
  --memory 4G \
  --cpu 1
```

#### Deployment Process

The deployment process automatically:

1. **Checks IBM Cloud Setup**
   - Verifies login status
   - Targets the specified region and resource group

2. **Manages Code Engine Plugin**
   - Checks if Code Engine plugin is installed
   - Installs it if missing

3. **Manages Projects**
   - Selects the specified Code Engine project
   - Provides helpful error messages if project doesn't exist

4. **Manages Secrets**
   - Creates or updates secrets from `.env` file
   - Uses the secret for environment variables

5. **Packages Application**
   - Copies source files to a temporary directory
   - Generates Dockerfile if not provided
   - Prepares for remote build

6. **Deploys Application**
   - Creates or updates the application
   - Uses remote build (no local Docker required)
   - Waits for deployment to complete
   - Shows application URL and endpoints

## Configuration

### Environment Variables

Create a `.env` file in your project root:

```env
WATSONX_API_KEY=your-api-key
WATSONX_PROJECT_ID=your-project-id
IAM_IBM_CLOUD_URL=iam.cloud.ibm.com
WATSONX_API_URL=https://us-south.ml.cloud.ibm.com
```

For Code Engine deployment, include any environment variables your app needs in the `.env` file. They will be automatically loaded as secrets.

### Default Values

If not specified, deployment uses:
- **App Name**: `watsonx-sdlc-bun`
- **Project Name**: `watsonx-sdlc-project`
- **Region**: `us-south`
- **Resource Group**: `Default`
- **Port**: `8000`
- **Memory**: `4G`
- **CPU**: `1`
- **Min Scale**: `1`
- **Max Scale**: `3`
- **Build Size**: `large`
- **Build Timeout**: `900` seconds

## Architecture

The implementation follows Domain-Driven Design (DDD):

- **Domain Layer**: Core business logic (deployment config, results, services)
- **Application Layer**: Use cases (DeployToCodeEngineUseCase)
- **Infrastructure Layer**: Implementation (CodeEngineDeploymentServiceImpl)

### Intent Detection

The system uses an `IntentDetector` to automatically detect:
- **Deployment requests**: "deploy to code engine", "deploy my app", etc.
- **Command translation**: Regular cloud CLI commands

This allows natural language to trigger the appropriate workflow.

## Examples

### Example 1: Simple Deployment

```bash
$ anycli
anycli> deploy my app to code engine
🚀 Detected deployment request
📦 App: watsonx-sdlc-bun, Project: watsonx-sdlc-project
❓ Execute this command? [Y/n]: y
🚀 Deploying to Code Engine (remote build)...
✅ Deployment successful!
🌐 Application URL: https://myapp-xxx.us-south.codeengine.appdomain.cloud
```

### Example 2: Command Translation

```bash
$ anycli
anycli> list all my databases
→ ibmcloud resource service-instances --service-name databases
❓ Execute this command? [Y/n]: y
🚀 Executing...
[command output]
✅ Command executed successfully
```

### Example 3: Deployment with Specific Names

```bash
$ anycli
anycli> deploy app named myapp to project myproject
🚀 Detected deployment request
📦 App: myapp, Project: myproject
❓ Execute this command? [Y/n]: y
...
```

## Troubleshooting

### Deployment Fails

1. **Check IBM Cloud Login**
   ```bash
   ibmcloud login
   ```

2. **Verify Project Exists**
   ```bash
   ibmcloud ce project list
   ```

3. **Check Plugin Installation**
   ```bash
   ibmcloud plugin list | grep code-engine
   ```

4. **Verify .env File**
   - Ensure `.env` file exists in project root
   - Check that required environment variables are set

### Command Translation Issues

1. **Improve RAG Context**: The system learns from corrections
2. **Be Specific**: More specific queries yield better results
3. **Use Provider Context**: Specify provider with `--provider` flag if needed

## Future Enhancements

- Support for other cloud platforms (AWS App Runner, GCP Cloud Run, Azure Container Apps)
- Multi-cloud deployment strategies
- Deployment history and rollback
- Advanced configuration options
- CI/CD integration

