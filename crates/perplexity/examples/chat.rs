//! Basic chat completion example.
//!
//! Run with:
//! ```bash
//! PERPLEXITY_API_KEY=your_key cargo run --example chat
//! ```

use perplexity::{ChatRequest, PerplexityClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("PERPLEXITY_API_KEY")
        .expect("PERPLEXITY_API_KEY environment variable must be set");

    // Create client
    let client = PerplexityClient::new(api_key)?;
    println!("Client created successfully");
    println!("Base URL: {}", client.base_url());

    // Create a simple chat request
    let request = ChatRequest::builder("llama-3.1-sonar-small-128k-online")
        .system("You are a helpful assistant. Be concise.")
        .user("What is Rust programming language?")
        .temperature(0.7)
        .max_tokens(500)
        .build();

    println!("\nSending request...\n");

    // Send the request
    match client.chat().complete(request).await {
        Ok(response) => {
            println!("=== Response ===");
            if let Some(content) = response.content() {
                println!("{}", content);
            }

            println!("\n=== Metadata ===");
            println!("Model: {}", response.model);
            println!("Usage: {} tokens", response.usage.total_tokens);

            // Print citations if available
            if response.has_citations() {
                println!("\n=== Citations ===");
                for citation in response.get_citations() {
                    if let Some(title) = &citation.title {
                        println!("[{}] {} - {}", citation.index, title, citation.url);
                    } else {
                        println!("[{}] {}", citation.index, citation.url);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
