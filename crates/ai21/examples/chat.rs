//! Example: Basic chat with AI21 Jamba models

use ai21::{Ai21Client, ChatRequest, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("AI21_API_KEY")
        .expect("AI21_API_KEY environment variable must be set");

    // Create client
    let client = Ai21Client::new(&api_key)?;

    // Create chat request
    let request = ChatRequest::builder("jamba-1.5-large")
        .messages(vec![
            Message::system("You are a helpful assistant."),
            Message::user("What is the capital of France?"),
        ])
        .temperature(0.7)
        .max_tokens(500)
        .build();

    // Send request
    println!("Sending request to AI21...");
    let response = client.chat().create(request).await?;

    // Print response
    println!("Response: {}", response.choices[0].message.content);
    println!("Model: {}", response.model);
    println!("Usage: {} tokens", response.usage.total_tokens);

    Ok(())
}
