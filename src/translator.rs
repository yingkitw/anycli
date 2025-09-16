use anyhow::Result;
use crate::watsonx::WatsonxAI;

pub struct CommandTranslator {
    watsonx: WatsonxAI,
}

impl CommandTranslator {
    pub fn new(watsonx: WatsonxAI) -> Self {
        Self { watsonx }
    }

    pub async fn translate(&self, query: &str) -> Result<String> {
        // Create prompt for command translation with more specific guidance
        let prompt = format!(
            "You are an expert IBM Cloud CLI assistant. Translate the following natural language query to an IBM Cloud CLI command.\n\n\
            Query: {}\n\n\
            Respond with ONLY the exact IBM Cloud command to run, with no additional text, explanations, or markdown.\n\
            The command should start with 'ibmcloud' and be a valid IBM Cloud CLI command.\n\
            \n\
            Important IBM Cloud CLI command guidelines:\n\
            - For listing resources use: 'ibmcloud resource service-instances [--service-name NAME]'\n\
            - For listing services catalog use: 'ibmcloud catalog service-marketplace'\n\
            - For WML instances use: 'ibmcloud resource service-instances --service-name watson-machine-learning'\n\
            - For Code Engine use: 'ibmcloud ce [subcommand]' or 'ibmcloud resource service-instances --service-name code-engine'\n\
            - For general resource listing use: 'ibmcloud resource service-instances' NOT 'services list'\n\
            \n\
            Make your best guess based on IBM Cloud CLI conventions and these guidelines.",
            query
        );
        
        // Get command translation from WatsonX
        let model_id = WatsonxAI::GRANITE_3_3_8B_INSTRUCT;
        let response = self.watsonx.watsonx_gen(&prompt, model_id, 200).await?;
        
        // Extract the first line that starts with "ibmcloud"
        let command = response.lines()
            .find(|line| line.trim().starts_with("ibmcloud"))
            .unwrap_or_else(|| {
                // If no line starts with "ibmcloud", use the first non-empty line
                response.lines()
                    .find(|line| !line.trim().is_empty())
                    .unwrap_or("")
            })
            .trim();
        
        // Ensure command starts with ibmcloud if not already
        let command = if !command.starts_with("ibmcloud ") && !command.is_empty() {
            format!("ibmcloud {}", command)
        } else if command.is_empty() {
            "ibmcloud".to_string()
        } else {
            command.to_string()
        };
        
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    
    #[tokio::test]
    async fn test_translate_simple_query() {
        // Skip test if no API key is available
        if env::var("WATSONX_API_KEY").is_err() && env::var("API_KEY").is_err() {
            return;
        }
        
        let mut watsonx = WatsonxAI::new().unwrap();
        watsonx.connect().await.unwrap();
        
        let translator = CommandTranslator::new(watsonx);
        let command = translator.translate("list all my cloud functions").await.unwrap();
        
        // Just check that we got something reasonable back
        assert!(command.contains("ibmcloud"));
        assert!(command.contains("fn") || command.contains("function"));
        assert!(command.contains("list"));
    }
}