//! Example: Streaming chat with Cohere

use cohere::{ChatRequest, CohereClient, StreamEvent};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("COHERE_API_KEY")
        .expect("COHERE_API_KEY environment variable must be set");

    // Create client
    let client = CohereClient::new(&api_key)?;

    // Create chat request
    let request = ChatRequest::builder("command-r")
        .message("Tell me a short story about a Rust programmer.")
        .build();

    // Send streaming request
    println!("Streaming response:\n");
    
    // Fix for Rust 2024 impl Trait lifetime capture rules
    let chat = client.chat();
    let mut stream = chat.stream(request).await?;

    while let Some(chunk) = stream.next().await {
        match chunk? {
            StreamEvent::StreamStart { generation_id } => {
                println!("[Stream started: {}]\n", generation_id);
            }
            StreamEvent::TextGeneration { text } => {
                print!("{}", text);
            }
            StreamEvent::StreamEnd { finish_reason, .. } => {
                println!("\n\n[Stream ended: {:?}]", finish_reason);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
