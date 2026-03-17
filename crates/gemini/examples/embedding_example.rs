//! Embedding example for Google Gemini SDK
//! 
//! Run with: cargo run --example embedding_example

use google_gemini::GeminiClient;
use google_gemini::embedding::{EmbeddingExt, cosine_similarity};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable must be set");
    
    let client = GeminiClient::new(api_key);
    
    // Get embeddings for texts
    println!("=== Getting Embeddings ===");
    
    let texts = vec![
        "The quick brown fox jumps over the lazy dog".to_string(),
        "A fast brown fox leaps over a sleepy dog".to_string(),
        "Machine learning is a subset of artificial intelligence".to_string(),
        "Rust is a systems programming language".to_string(),
    ];
    
    let embeddings = client.embed_builder().embed_texts(texts.clone()).await?;
    
    println!("Generated {} embeddings", embeddings.len());
    
    // Calculate similarity between first two texts (should be high)
    let sim_0_1 = cosine_similarity(&embeddings[0].values, &embeddings[1].values);
    println!("\nSimilarity between '{}' and '{}'", texts[0], texts[1]);
    println!("Score: {:.4}", sim_0_1);
    
    // Calculate similarity between first and third (should be lower)
    let sim_0_2 = cosine_similarity(&embeddings[0].values, &embeddings[2].values);
    println!("\nSimilarity between '{}' and '{}'", texts[0], texts[2]);
    println!("Score: {:.4}", sim_0_2);
    
    // Single embedding
    println!("\n=== Single Embedding ===");
    let embedding = client.embed_text_simple("Hello, world!").await?;
    println!("Embedding dimension: {}", embedding.values.len());
    
    Ok(())
}
