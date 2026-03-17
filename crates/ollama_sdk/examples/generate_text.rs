//! Example: Generate completion with Ollama
//!
//! Run with: cargo run --example generate_text

use ollama_sdk::{GenerateRequest, OllamaClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Create client with default local Ollama instance
    let client = OllamaClient::new("http://localhost:11434");

    println!("Sending generate request to Ollama...\n");

    // Build a generate request (single-turn completion)
    let request = GenerateRequest::builder("llama3.2")
        .prompt("The quick brown fox")
        .system("Complete the sentence concisely.")
        .temperature(0.8)
        .max_tokens(50)
        .build();

    // Send the request and get response
    match client.generate().text(request).await {
        Ok(response) => {
            println!("Model: {}", response.model);
            println!("Prompt: 'The quick brown fox'");
            println!("Completion: {}", response.response);
            
            if let Some(eval_count) = response.eval_count {
                println!("\nTokens generated: {}", eval_count);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    // Example with JSON mode
    println!("\n--- JSON Mode Example ---\n");

    let request = GenerateRequest::builder("llama3.2")
        .prompt("Generate a JSON object with 'name' and 'age' fields for a fictional character")
        .system("You are a JSON generator. Respond only with valid JSON.")
        .json_mode()
        .build();

    match client.generate().text(request).await {
        Ok(response) => {
            println!("Response: {}", response.response);
            
            // Try to parse as JSON
            match serde_json::from_str::<serde_json::Value>(&response.response) {
                Ok(json) => println!("Parsed JSON: {}", serde_json::to_string_pretty(&json)?),
                Err(e) => println!("Failed to parse JSON: {}", e),
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}
