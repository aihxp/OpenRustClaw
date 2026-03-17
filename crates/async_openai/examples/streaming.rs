//! Streaming chat completion example.
//!
//! Run with:
//! ```bash
//! OPENAI_API_KEY=your-key cargo run --example streaming --features streaming
//! ```

use futures::StreamExt;

use async_openai::{ChatRequest, OpenAIClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key =
        std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY environment variable not set");

    let client = OpenAIClient::new(api_key)?;

    let request = ChatRequest::builder("gpt-4o-mini")
        .user("Count from 1 to 10 slowly.")
        .stream(true)
        .build();

    println!("Streaming response:\n");

    let chat = client.chat();
    let mut stream = chat.stream(request).await?;
    while let Some(result) = stream.next().await {
        match result {
            Ok(chunk) => {
                if chunk.is_done() {
                    break;
                }
                if let Some(content) = chunk.content() {
                    print!("{}", content);
                }
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }

    println!();
    Ok(())
}
