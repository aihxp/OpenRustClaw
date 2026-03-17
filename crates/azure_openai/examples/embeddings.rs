//! Embeddings example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_EMBEDDING_DEPLOYMENT=your-embedding-deployment \
//! cargo run --example embeddings
//! ```

use azure_openai::{AzureOpenAIClient, EmbeddingRequest};
use azure_openai::embeddings::EmbeddingExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_EMBEDDING_DEPLOYMENT")
        .expect("AZURE_OPENAI_EMBEDDING_DEPLOYMENT environment variable not set");

    // Create the client
    let client = AzureOpenAIClient::new(
        &resource_name,
        &deployment_name,
        &api_key,
    )?;

    println!("Azure OpenAI Embeddings Example\n");

    // Single embedding
    println!("1. Single Embedding:");
    let text = "The quick brown fox jumps over the lazy dog";
    let embedding = client.embeddings().embed(text).await?;
    println!("   Text: {}", text);
    println!("   Dimensions: {}", embedding.len());
    println!("   First 5 values: {:?}", &embedding[..5.min(embedding.len())]);

    // Multiple embeddings
    println!("\n2. Multiple Embeddings:");
    let texts = vec![
        "Rust is a systems programming language".to_string(),
        "Python is great for data science".to_string(),
        "JavaScript runs in the browser".to_string(),
        "Rust provides memory safety without garbage collection".to_string(),
    ];

    let request = EmbeddingRequest::new(texts.clone());
    let response = client.embeddings().create(request).await?;

    println!("   Generated {} embeddings", response.data.len());

    // Calculate similarity between embeddings
    println!("\n3. Cosine Similarities:");
    let embeddings: Vec<_> = response.data.iter().map(|e| e.embedding.clone()).collect();

    for i in 0..embeddings.len() {
        for j in (i + 1)..embeddings.len() {
            let similarity = embeddings[i].cosine_similarity(&embeddings[j]);
            println!(
                "   '{}' <-> '{}': {:.4}",
                truncate(&texts[i], 30),
                truncate(&texts[j], 30),
                similarity
            );
        }
    }

    // Reduce dimensions
    println!("\n4. Reduced Dimensions (512):");
    let request_512 = EmbeddingRequest::new(vec!["Test text".to_string()]).dimensions(512);
    let response_512 = client.embeddings().create(request_512).await?;
    if let Some(embedding) = response_512.first() {
        println!("   Dimensions: {}", embedding.dimensions());
    }

    println!("\nDone!");
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
