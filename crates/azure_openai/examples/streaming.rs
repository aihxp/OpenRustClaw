//! Streaming chat completion example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DEPLOYMENT=your-deployment \
//! cargo run --example streaming
//! ```

use azure_openai::{AzureOpenAIClient, ChatRequest, Role};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
        .expect("AZURE_OPENAI_DEPLOYMENT environment variable not set");

    // Create the client
    let client = AzureOpenAIClient::new(
        &resource_name,
        &deployment_name,
        api_key.clone(),
    )?;

    println!("Sending streaming chat completion request...\n");

    // Build the request
    let request = ChatRequest::builder()
        .system("You are a creative storyteller.")
        .user("Tell me a short story about a robot learning to paint.")
        .temperature(0.8)
        .max_tokens(300)
        .build();

    // Send the streaming request
    let chat = client.chat();
    let mut stream = chat.stream(request).await?;

    println!("Response:\n");
    let mut full_content = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                if chunk.is_done() {
                    break;
                }

                if let Some(content) = chunk.content() {
                    print!("{}", content);
                    full_content.push_str(content);
                    // Flush stdout for immediate display
                    std::io::Write::flush(&mut std::io::stdout())?;
                }

                if let Some(tool_calls) = chunk.tool_calls() {
                    for tool_call in tool_calls {
                        println!("\n[Tool call detected: {:?}]", tool_call);
                    }
                }
            }
            Err(e) => {
                eprintln!("\n\nError: {}", e);
                break;
            }
        }
    }

    println!("\n\n---");
    println!("Total characters received: {}", full_content.len());

    Ok(())
}
