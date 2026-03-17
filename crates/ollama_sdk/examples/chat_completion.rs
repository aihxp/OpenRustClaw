//! Example: Basic chat completion with Ollama
//!
//! Run with: cargo run --example chat_completion

use ollama_sdk::{ChatRequest, OllamaClient, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Create client with default local Ollama instance
    let client = OllamaClient::new("http://localhost:11434");

    println!("Sending chat request to Ollama...\n");

    // Build a chat request
    let request = ChatRequest::builder("llama3.2")
        .system("You are a helpful, concise assistant.")
        .message(Role::User, "What is Rust programming language?")
        .temperature(0.7)
        .build();

    // Send the request and get response
    match client.chat().generate(request).await {
        Ok(response) => {
            println!("Model: {}", response.model);
            println!("Response: {}", response.content());
            
            if let Some(eval_count) = response.eval_count {
                println!("\nTokens generated: {}", eval_count);
            }
            if let Some(prompt_eval) = response.prompt_eval_count {
                println!("Prompt tokens: {}", prompt_eval);
            }
            if let Some(total_dur) = response.total_duration {
                println!("Total time: {:.2}s", total_dur as f64 / 1_000_000_000.0);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
