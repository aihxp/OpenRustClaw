//! Chat completion example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DEPLOYMENT=your-deployment \
//! cargo run --example chat_completion
//! ```

use azure_openai::{AzureOpenAIClient, ChatRequest, Role};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
        .expect("AZURE_OPENAI_DEPLOYMENT environment variable not set");

    // Create the client
    let client = AzureOpenAIClient::new(&resource_name, &deployment_name, api_key.clone())?;

    println!("Sending chat completion request...\n");

    // Build the request
    let request = ChatRequest::builder()
        .system("You are a helpful assistant specialized in Rust programming.")
        .user("What are the key features of Rust's ownership system?")
        .temperature(0.7)
        .max_tokens(500)
        .build();

    // Send the request
    let response = client.chat().complete(request).await?;

    println!("Response ID: {}", response.id);
    println!("Model: {}", response.model);
    println!("\nContent:\n{}", response.content().unwrap_or("No content"));
    println!("\nUsage: {} tokens", response.usage.total_tokens);
    println!("  - Prompt tokens: {}", response.usage.prompt_tokens);
    println!(
        "  - Completion tokens: {}",
        response.usage.completion_tokens
    );

    // Check for content filter results
    if let Some(filter_results) = response
        .choices
        .first()
        .and_then(|c| c.content_filter_results.as_ref())
    {
        println!("\nContent Filter Results:");
        if let Some(hate) = &filter_results.hate {
            println!(
                "  Hate: filtered={}, severity={:?}",
                hate.filtered, hate.severity
            );
        }
        if let Some(self_harm) = &filter_results.self_harm {
            println!(
                "  Self-harm: filtered={}, severity={:?}",
                self_harm.filtered, self_harm.severity
            );
        }
        if let Some(sexual) = &filter_results.sexual {
            println!(
                "  Sexual: filtered={}, severity={:?}",
                sexual.filtered, sexual.severity
            );
        }
        if let Some(violence) = &filter_results.violence {
            println!(
                "  Violence: filtered={}, severity={:?}",
                violence.filtered, violence.severity
            );
        }
    }

    Ok(())
}
