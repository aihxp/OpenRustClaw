//! Assistants API for Azure OpenAI.

use crate::client::AzureOpenAIClient;
use crate::error::Result;
use crate::types::Function;

/// Client for the assistants API.
#[derive(Debug)]
pub struct Assistants<'a> {
    client: &'a AzureOpenAIClient,
}

impl<'a> Assistants<'a> {
    /// Create a new assistants client.
    pub fn new(client: &'a AzureOpenAIClient) -> Self {
        Self { client }
    }

    /// Create a new assistant.
    pub async fn create(&self, request: AssistantRequest) -> Result<Assistant> {
        let body = serde_json::to_value(&request)?;

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post("/openai/assistants", body).await?;
                    let body = client.handle_response(response).await?;
                    let assistant: Assistant = serde_json::from_value(body)?;
                    Ok(assistant)
                })
            })
            .await
    }

    /// Retrieve an assistant.
    pub async fn retrieve(&self, assistant_id: &str) -> Result<Assistant> {
        let path = format!("{}/{}", "/openai/assistants", assistant_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let assistant: Assistant = serde_json::from_value(body)?;
                    Ok(assistant)
                })
            })
            .await
    }

    /// Update an assistant.
    pub async fn update(&self, assistant_id: &str, request: AssistantRequest) -> Result<Assistant> {
        let body = serde_json::to_value(&request)?;
        let path = format!("{}/{}", "/openai/assistants", assistant_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.post(&path, body).await?;
                    let body = client.handle_response(response).await?;
                    let assistant: Assistant = serde_json::from_value(body)?;
                    Ok(assistant)
                })
            })
            .await
    }

    /// Delete an assistant.
    pub async fn delete(&self, assistant_id: &str) -> Result<DeletionStatus> {
        let path = format!("{}/{}", "/openai/assistants", assistant_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.delete(&path).await?;
                    let body = client.handle_response(response).await?;
                    let status: DeletionStatus = serde_json::from_value(body)?;
                    Ok(status)
                })
            })
            .await
    }

    /// List assistants.
    pub async fn list(&self, limit: Option<usize>) -> Result<AssistantList> {
        let mut path = "/openai/assistants".to_string();
        if let Some(limit) = limit {
            path.push_str(&format!("?limit={}", limit));
        }

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let list: AssistantList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// Create a thread.
    pub async fn create_thread(&self, request: Option<ThreadRequest>) -> Result<Thread> {
        let body = request
            .map(|r| serde_json::to_value(&r).ok())
            .flatten()
            .unwrap_or_else(|| serde_json::json!({}));

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                Box::pin(async move {
                    let response = client.post("/openai/threads", body).await?;
                    let body = client.handle_response(response).await?;
                    let thread: Thread = serde_json::from_value(body)?;
                    Ok(thread)
                })
            })
            .await
    }

    /// Retrieve a thread.
    pub async fn retrieve_thread(&self, thread_id: &str) -> Result<Thread> {
        let path = format!("{}/{}", "/openai/threads", thread_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let thread: Thread = serde_json::from_value(body)?;
                    Ok(thread)
                })
            })
            .await
    }

    /// Delete a thread.
    pub async fn delete_thread(&self, thread_id: &str) -> Result<DeletionStatus> {
        let path = format!("{}/{}", "/openai/threads", thread_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.delete(&path).await?;
                    let body = client.handle_response(response).await?;
                    let status: DeletionStatus = serde_json::from_value(body)?;
                    Ok(status)
                })
            })
            .await
    }

    /// Create a message in a thread.
    pub async fn create_message(
        &self,
        thread_id: &str,
        request: ThreadMessageRequest,
    ) -> Result<ThreadMessage> {
        let body = serde_json::to_value(&request)?;
        let path = format!("{}/{}/messages", "/openai/threads", thread_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.post(&path, body).await?;
                    let body = client.handle_response(response).await?;
                    let message: ThreadMessage = serde_json::from_value(body)?;
                    Ok(message)
                })
            })
            .await
    }

    /// List messages in a thread.
    pub async fn list_messages(&self, thread_id: &str) -> Result<MessageList> {
        let path = format!("{}/{}/messages", "/openai/threads", thread_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let list: MessageList = serde_json::from_value(body)?;
                    Ok(list)
                })
            })
            .await
    }

    /// Create a run.
    pub async fn create_run(&self, thread_id: &str, request: RunRequest) -> Result<Run> {
        let body = serde_json::to_value(&request)?;
        let path = format!("{}/{}/runs", "/openai/threads", thread_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let body = body.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.post(&path, body).await?;
                    let body = client.handle_response(response).await?;
                    let run: Run = serde_json::from_value(body)?;
                    Ok(run)
                })
            })
            .await
    }

    /// Retrieve a run.
    pub async fn retrieve_run(&self, thread_id: &str, run_id: &str) -> Result<Run> {
        let path = format!("{}/{}/runs/{}", "/openai/threads", thread_id, run_id);

        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                let path = path.clone();
                Box::pin(async move {
                    let response = client.get(&path).await?;
                    let body = client.handle_response(response).await?;
                    let run: Run = serde_json::from_value(body)?;
                    Ok(run)
                })
            })
            .await
    }
}

/// An assistant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Assistant {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The creation timestamp.
    pub created_at: i64,
    /// The name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The model.
    pub model: String,
    /// The instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// The tools.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AssistantTool>,
    /// The file IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_ids: Vec<String>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// An assistant tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum AssistantTool {
    /// Code interpreter tool.
    CodeInterpreter,
    /// Retrieval tool.
    Retrieval,
    /// Function tool.
    Function { function: Function },
}

/// An assistant request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantRequest {
    /// The model.
    pub model: String,
    /// The name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// The tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AssistantTool>>,
    /// The file IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl AssistantRequest {
    /// Create a new assistant request.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            name: None,
            description: None,
            instructions: None,
            tools: None,
            file_ids: None,
            metadata: None,
        }
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the instructions.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Add a tool.
    pub fn add_tool(mut self, tool: AssistantTool) -> Self {
        self.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Enable code interpreter.
    pub fn enable_code_interpreter(mut self) -> Self {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(AssistantTool::CodeInterpreter);
        self
    }

    /// Enable retrieval.
    pub fn enable_retrieval(mut self) -> Self {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(AssistantTool::Retrieval);
        self
    }

    /// Add a function tool.
    pub fn add_function(mut self, function: Function) -> Self {
        self.tools
            .get_or_insert_with(Vec::new)
            .push(AssistantTool::Function { function });
        self
    }

    /// Set the file IDs.
    pub fn file_ids(mut self, file_ids: Vec<String>) -> Self {
        self.file_ids = Some(file_ids);
        self
    }
}

/// An assistant list response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssistantList {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<Assistant>,
    /// The first ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    /// The last ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
    /// Whether there are more items.
    pub has_more: bool,
}

/// A thread.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Thread {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The creation timestamp.
    pub created_at: i64,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A thread request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreadRequest {
    /// The messages to start with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<ThreadMessageRequest>>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// A thread message.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreadMessage {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The creation timestamp.
    pub created_at: i64,
    /// The thread ID.
    pub thread_id: String,
    /// The role.
    pub role: String,
    /// The content.
    pub content: Vec<MessageContent>,
    /// The assistant ID (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_id: Option<String>,
    /// The run ID (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// The file IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_ids: Vec<String>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Message content.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum MessageContent {
    /// Text content.
    Text {
        /// The text value.
        text: TextContent,
    },
    /// Image file content.
    ImageFile {
        /// The image file.
        image_file: ImageFileContent,
    },
}

/// Text content.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextContent {
    /// The value.
    pub value: String,
    /// The annotations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub annotations: Vec<serde_json::Value>,
}

/// Image file content.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImageFileContent {
    /// The file ID.
    pub file_id: String,
}

/// A thread message request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThreadMessageRequest {
    /// The role.
    pub role: String,
    /// The content.
    pub content: String,
    /// The file IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl ThreadMessageRequest {
    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            file_ids: None,
            metadata: None,
        }
    }
}

/// A message list response.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageList {
    /// The object type.
    pub object: String,
    /// The data.
    pub data: Vec<ThreadMessage>,
    /// The first ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    /// The last ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
    /// Whether there are more items.
    pub has_more: bool,
}

/// A run.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Run {
    /// The identifier.
    pub id: String,
    /// The object type.
    pub object: String,
    /// The creation timestamp.
    pub created_at: i64,
    /// The assistant ID.
    pub assistant_id: String,
    /// The thread ID.
    pub thread_id: String,
    /// The status.
    pub status: RunStatus,
    /// When the run was started.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    /// When the run expired.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    /// When the run was cancelled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<i64>,
    /// When the run failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<i64>,
    /// When the run completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<i64>,
    /// The last error (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<RunError>,
    /// The model.
    pub model: String,
    /// The instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// The tools.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AssistantTool>,
    /// The file IDs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_ids: Vec<String>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Required action (if any).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_action: Option<RequiredAction>,
}

/// Run status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    /// The run is queued.
    Queued,
    /// The run is in progress.
    InProgress,
    /// The run requires action.
    RequiresAction,
    /// The run is cancelling.
    Cancelling,
    /// The run is cancelled.
    Cancelled,
    /// The run failed.
    Failed,
    /// The run completed.
    Completed,
    /// The run expired.
    Expired,
}

/// A run error.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunError {
    /// The error code.
    pub code: String,
    /// The error message.
    pub message: String,
}

/// A required action.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RequiredAction {
    /// The type.
    #[serde(rename = "type")]
    pub action_type: String,
    /// Submit tool outputs.
    #[serde(rename = "submit_tool_outputs")]
    pub submit_tool_outputs: SubmitToolOutputs,
}

/// Submit tool outputs.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SubmitToolOutputs {
    /// The tool calls.
    pub tool_calls: Vec<ToolCallOutput>,
}

/// A tool call output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallOutput {
    /// The ID.
    pub id: String,
    /// The type.
    #[serde(rename = "type")]
    pub call_type: String,
    /// The function.
    pub function: FunctionOutput,
}

/// A function output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionOutput {
    /// The name.
    pub name: String,
    /// The arguments.
    pub arguments: String,
}

/// A run request.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RunRequest {
    /// The assistant ID.
    pub assistant_id: String,
    /// The model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Additional instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_instructions: Option<String>,
    /// The tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AssistantTool>>,
    /// The metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl RunRequest {
    /// Create a new run request.
    pub fn new(assistant_id: impl Into<String>) -> Self {
        Self {
            assistant_id: assistant_id.into(),
            model: None,
            instructions: None,
            additional_instructions: None,
            tools: None,
            metadata: None,
        }
    }
}

/// Deletion status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeletionStatus {
    /// The ID.
    pub id: String,
    /// The object type.
    pub object: String,
    /// Whether the deletion was successful.
    pub deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assistant_request_builder() {
        let request = AssistantRequest::new("gpt-4")
            .name("My Assistant")
            .instructions("You are helpful")
            .enable_code_interpreter();

        assert_eq!(request.model, "gpt-4");
        assert_eq!(request.name, Some("My Assistant".to_string()));
        assert_eq!(request.instructions, Some("You are helpful".to_string()));
        assert!(request.tools.is_some());
    }

    #[test]
    fn test_thread_message_request() {
        let request = ThreadMessageRequest::user("Hello!");
        assert_eq!(request.role, "user");
        assert_eq!(request.content, "Hello!");
    }

    #[test]
    fn test_run_request() {
        let request = RunRequest::new("asst_123");
        assert_eq!(request.assistant_id, "asst_123");
    }

    #[test]
    fn test_run_status() {
        let status = RunStatus::Completed;
        assert_eq!(serde_json::to_string(&status).unwrap(), "\"completed\"");
    }
}
