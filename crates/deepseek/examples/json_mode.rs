//! JSON mode example
//!
//! Run with: cargo run --example json_mode
//!
//! This example demonstrates how to use JSON mode for structured output.

use deepseek::{ChatRequest, DeepSeekClient, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debugging
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY environment variable must be set");

    // Create client
    let client = DeepSeekClient::new(api_key)?;

    // Create a JSON mode request
    // Note: You must instruct the model to produce JSON in the system or user message
    let request = ChatRequest::builder("deepseek-chat")
        .system(
            "You are a helpful assistant that always responds with valid JSON. \
             Do not include markdown formatting, just the raw JSON object.",
        )
        .message(
            Role::User,
            "Generate a list of 3 programming languages with their creation year and creator. \
             Format as JSON with this structure: {\"languages\": [{\"name\": \"...\", \"year\": ..., \"creator\": \"...\"}]}",
        )
        .json_mode()
        .temperature(0.3)
        .build();

    // Send the request
    println!("Sending JSON mode request...\n");
    let response = client.chat().complete(request).await?;

    // Parse and print the JSON response
    if let Some(content) = response.content() {
        println!("Raw response:\n{}\n", content);

        // Parse the JSON
        match serde_json::from_str::<serde_json::Value>(content) {
            Ok(json) => {
                println!("Parsed JSON:");
                println!("{}", serde_json::to_string_pretty(&json)?);

                // Access specific fields
                if let Some(languages) = json.get("languages").and_then(|l| l.as_array()) {
                    println!("\nLanguages found: {}", languages.len());
                    for lang in languages {
                        if let (Some(name), Some(year), Some(creator)) = (
                            lang.get("name").and_then(|n| n.as_str()),
                            lang.get("year").and_then(|y| y.as_u64()),
                            lang.get("creator").and_then(|c| c.as_str()),
                        ) {
                            println!("  - {} ({}) - Created by {}", name, year, creator);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to parse JSON: {}", e);
                eprintln!("Raw content: {}", content);
            }
        }
    }

    // Print token usage
    println!("\n--- Token Usage ---");
    println!("Prompt tokens: {}", response.usage.prompt_tokens);
    println!("Completion tokens: {}", response.usage.completion_tokens);
    println!("Total tokens: {}", response.usage.total_tokens);

    Ok(())
}
