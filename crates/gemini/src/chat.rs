//! Chat session for multi-turn conversations

use crate::{
    Content, GeminiClient, GeminiError, GenerateContentRequest, GenerateContentResponse,
    GenerationConfig, Part,
};

/// Chat session with history
pub struct ChatSession<'a> {
    client: &'a GeminiClient,
    history: Vec<Content>,
    system_instruction: Option<Content>,
    generation_config: Option<GenerationConfig>,
    tools: Option<Vec<crate::types::Tool>>,
    tool_config: Option<crate::types::ToolConfig>,
}

impl<'a> ChatSession<'a> {
    /// Create new chat session
    pub fn new(client: &'a GeminiClient) -> Self {
        Self {
            client,
            history: vec![],
            system_instruction: None,
            generation_config: None,
            tools: None,
            tool_config: None,
        }
    }

    /// Set system instruction
    pub fn with_system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.system_instruction = Some(Content::system(instruction));
        self
    }

    /// Set generation config
    pub fn with_generation_config(mut self, config: GenerationConfig) -> Self {
        self.generation_config = Some(config);
        self
    }

    /// Set tools for function calling
    pub fn with_tools(mut self, tools: Vec<crate::types::Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set tool config
    pub fn with_tool_config(mut self, config: crate::types::ToolConfig) -> Self {
        self.tool_config = Some(config);
        self
    }

    /// Send message and get response
    pub async fn send_message(
        &mut self,
        message: impl Into<String>,
    ) -> Result<String, GeminiError> {
        let message_text: String = message.into();
        let user_content = Content::user(&message_text);

        let mut contents = self.history.clone();
        contents.push(user_content);

        let request = GenerateContentRequest {
            contents: contents.clone(),
            system_instruction: self.system_instruction.clone(),
            generation_config: self.generation_config.clone(),
            tools: self.tools.clone(),
            tool_config: self.tool_config.clone(),
            safety_settings: None,
        };

        let response = self.client.generate_content(request).await?;

        // Extract text from response
        let text = self.extract_text(&response);

        // Update history
        if let Some(last_req) = contents.last() {
            self.history.push(last_req.clone());
        }
        if let Some(content) = response.candidates.first().map(|c| c.content.clone()) {
            self.history.push(content);
        }

        Ok(text)
    }

    /// Send message with image
    pub async fn send_message_with_image(
        &mut self,
        message: impl Into<String>,
        mime_type: &str,
        image_data: Vec<u8>,
    ) -> Result<String, GeminiError> {
        let message_text: String = message.into();
        let user_content = Content::user(&message_text).with_image(mime_type, image_data);

        let mut contents = self.history.clone();
        contents.push(user_content);

        let request = GenerateContentRequest {
            contents: contents.clone(),
            system_instruction: self.system_instruction.clone(),
            generation_config: self.generation_config.clone(),
            tools: self.tools.clone(),
            tool_config: self.tool_config.clone(),
            safety_settings: None,
        };

        let response = self.client.generate_content(request).await?;
        let text = self.extract_text(&response);

        // Update history
        if let Some(last_req) = contents.last() {
            self.history.push(last_req.clone());
        }
        if let Some(content) = response.candidates.first().map(|c| c.content.clone()) {
            self.history.push(content);
        }

        Ok(text)
    }

    /// Send a message and get the full response
    pub async fn send_message_full(
        &mut self,
        message: impl Into<String>,
    ) -> Result<GenerateContentResponse, GeminiError> {
        let message_text: String = message.into();
        let user_content = Content::user(&message_text);

        let mut contents = self.history.clone();
        contents.push(user_content);

        let request = GenerateContentRequest {
            contents: contents.clone(),
            system_instruction: self.system_instruction.clone(),
            generation_config: self.generation_config.clone(),
            tools: self.tools.clone(),
            tool_config: self.tool_config.clone(),
            safety_settings: None,
        };

        let response = self.client.generate_content(request).await?;

        // Update history
        if let Some(last_req) = contents.last() {
            self.history.push(last_req.clone());
        }
        if let Some(content) = response.candidates.first().map(|c| c.content.clone()) {
            self.history.push(content);
        }

        Ok(response)
    }

    /// Extract text from response
    fn extract_text(&self, response: &GenerateContentResponse) -> String {
        response
            .candidates
            .iter()
            .filter_map(|c| {
                c.content
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        Part::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .next()
            })
            .collect::<Vec<_>>()
            .join("")
    }

    /// Get chat history
    pub fn history(&self) -> &[Content] {
        &self.history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// Export history
    pub fn export_history(&self) -> Vec<Content> {
        self.history.clone()
    }

    /// Import history
    pub fn import_history(&mut self, history: Vec<Content>) {
        self.history = history;
    }

    /// Get token count for current history
    pub async fn count_tokens(&self) -> Result<i32, GeminiError> {
        let response = self.client.count_tokens(self.history.clone()).await?;
        Ok(response.total_tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_session_creation() {
        let client = GeminiClient::new("test-key");
        let session = client.chat();
        assert!(session.history.is_empty());
    }

    #[test]
    fn test_chat_with_system_instruction() {
        let client = GeminiClient::new("test-key");
        let session = client
            .chat()
            .with_system_instruction("You are a helpful assistant.");
        assert!(session.system_instruction.is_some());
    }
}
