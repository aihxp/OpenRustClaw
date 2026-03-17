//! Example: Embeddings with Cohere

use cohere::{EmbedRequest, CohereClient, InputType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("COHERE_API_KEY")
        .expect("COHERE_API_KEY environment variable must be set");

    // Create client
    let client = CohereClient::new(&api_key)?;

    // Create embeddings request
    let request = EmbedRequest::builder()
        .model("embed-english-v3.0")
        .add_text("Hello, how are you?")
        .add_text("The weather is nice today.")
        .add_text("Rust is a systems programming language.")
        .input_type(InputType::SearchDocument)
        .build();

    // Send request
    println!("Generating embeddings...");
    let response = client.embed().create(request).await?;

    // Print results
    println!("Generated {} embeddings:\n", response.len());

    for (i, (text, embedding)) in response.texts.iter().zip(response.get_embeddings()).enumerate() {
        println!("Text {}: {}", i + 1, text);
        println!("  Dimensions: {}", embedding.len());
        println!("  First 5 values: {:?}\n", &embedding[..5.min(embedding.len())]);
    }

    Ok(())
}
