//! Example: Create a custom model with Ollama
//!
//! Run with: cargo run --example create_model
//!
//! This example demonstrates how to create a custom model using a Modelfile.
//! See: https://github.com/ollama/ollama/blob/main/docs/modelfile.md

use futures::StreamExt;
use ollama_sdk::{ChatRequest, OllamaClient, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Create client with default local Ollama instance
    let client = OllamaClient::new("http://localhost:11434");

    // Define a simple Modelfile
    let modelfile = r#"
FROM llama3.2

SYSTEM """
You are a Rust programming expert. Always provide:
1. Type-safe code examples
2. Memory safety explanations
3. Idiomatic Rust patterns

Be concise but thorough.
"""

PARAMETER temperature 0.7
PARAMETER top_p 0.9
"#;

    let model_name = "rust-expert";

    println!("Creating custom model '{}'...\n", model_name);
    println!("Modelfile:\n{}\n", modelfile);

    // Create the model with streaming progress
    let mut stream = client.models().create(model_name, modelfile).await?;

    while let Some(status) = stream.next().await {
        match status {
            Ok(status) => {
                println!("Status: {}", status.status);
                if status.is_complete() {
                    println!("\n✅ Model creation complete!");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Test the newly created model
    println!("\n=== Testing the new model ===\n");

    let request = ChatRequest::builder(model_name)
        .message(Role::User, "Explain ownership in Rust with an example.")
        .build();

    match client.chat().generate(request).await {
        Ok(response) => {
            println!("Response from '{}' model:\n", model_name);
            println!("{}\n", response.content());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    // Show model info
    println!("=== Model Info ===\n");
    match client.models().show(model_name).await {
        Ok(info) => {
            if let Some(system) = info.system {
                println!("System prompt (first 200 chars):");
                println!("{}\n", &system[..system.len().min(200)]);
            }
            if let Some(template) = info.template {
                println!("Template length: {} characters", template.len());
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    // Clean up - delete the test model
    println!("\n=== Cleaning up ===");
    match client.models().delete(model_name).await {
        Ok(_) => println!("✅ Deleted model '{}'", model_name),
        Err(e) => eprintln!("Error deleting model: {}", e),
    }

    Ok(())
}
