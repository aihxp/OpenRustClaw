//! Example: Basic chat with Cohere

use cohere::{ChatRequest, CohereClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("COHERE_API_KEY")
        .expect("COHERE_API_KEY environment variable must be set");

    // Create client
    let client = CohereClient::new(&api_key)?;

    // Create chat request
    let request = ChatRequest::builder("command-r")
        .message("What is the capital of France?")
        .temperature(0.7)
        .build();

    // Send request
    println!("Sending request to Cohere...");
    let response = client.chat().create(request).await?;

    // Print response
    println!("Response: {}", response.text());
    println!("Generation ID: {}", response.generation_id);

    Ok(())
}
