//! Example: Text completion with AI21 Jurassic models

use ai21::{Ai21Client, CompletionRequest, Penalty};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("AI21_API_KEY")
        .expect("AI21_API_KEY environment variable must be set");

    // Create client
    let client = Ai21Client::new(api_key)?;

    // Create completion request
    let request = CompletionRequest::builder("j2-ultra")
        .prompt("The capital of France is")
        .max_tokens(50)
        .temperature(0.7)
        .presence_penalty(Penalty::new(0.5))
        .build();

    // Send request
    println!("Sending completion request to AI21...");
    let response = client.completions().create(request).await?;

    // Print response
    if let Some(text) = response.text() {
        println!("Completion: {}", text);
    }
    println!("Prompt tokens: {}", response.prompt.tokens.len());
    println!("Completion tokens: {}", response.completions[0].tokens.len());

    Ok(())
}
