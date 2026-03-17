//! Example: Streaming chat completion with Ollama
//!
//! Run with: cargo run --example streaming_chat

use futures::StreamExt;
use ollama_sdk::{ChatRequest, OllamaClient, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Create client with default local Ollama instance
    let client = OllamaClient::new("http://localhost:11434");

    println!("Streaming chat response from Ollama...\n");

    // Build a chat request
    let request = ChatRequest::builder("llama3.2")
        .system("You are a helpful assistant.")
        .message(Role::User, "Write a haiku about programming in Rust")
        .build();

    // Stream the response
    let mut stream = client.chat().stream(request).await?;
    let mut full_response = String::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(chunk) => {
                if let Some(content) = &chunk.message.content {
                    print!("{}", content);
                    full_response.push_str(content);
                    std::io::Write::flush(&mut std::io::stdout())?;
                }

                if chunk.done {
                    println!("\n\n[Stream complete]");
                    if let Some(eval_count) = chunk.eval_count {
                        println!("Tokens generated: {}", eval_count);
                    }
                }
            }
            Err(e) => {
                eprintln!("\nError: {}", e);
                break;
            }
        }
    }

    println!("\nFull response:\n{}", full_response);

    Ok(())
}
