//! Simple completion example using anthropic-rust.
//!
//! Run with:
//! ```bash
//! ANTHROPIC_API_KEY=your-key cargo run --example simple_completion
//! ```

use anthropic_rust::{AnthropicClient, MessageRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load API key from environment
    let api_key =
        std::env::var("ANTHROPIC_API_KEY").expect("ANTHROPIC_API_KEY environment variable not set");

    // Create client
    let client = AnthropicClient::new(api_key)?;

    // Create a simple request
    let request = MessageRequest::builder("claude-3-5-sonnet-20241022")
        .system("You are a helpful assistant that gives concise answers.")
        .user("What is the capital of France?")
        .max_tokens(1024)
        .temperature(0.7)
        .build();

    println!("Sending request...\n");

    // Send the request
    let response = client.messages().create(request).await?;

    // Print the response
    println!("Response ID: {}", response.id);
    println!("Model: {}", response.model);
    println!("\nContent:\n{}", response.text());
    println!("\nUsage:");
    println!("  Input tokens: {}", response.usage.input_tokens);
    println!("  Output tokens: {}", response.usage.output_tokens);
    println!("  Total tokens: {}", response.usage.total_tokens());

    Ok(())
}
