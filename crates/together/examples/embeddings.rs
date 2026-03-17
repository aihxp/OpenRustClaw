//! Example: Embeddings with Together AI.
//!
//! Run with:
//! ```
//! TOGETHER_API_KEY=your-api-key cargo run --example embeddings --features embeddings
//! ```

use together_ai::{
    TogetherClient,
    embeddings::{EmbeddingRequest, cosine_similarity},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key =
        std::env::var("TOGETHER_API_KEY").expect("TOGETHER_API_KEY environment variable not set");

    // Create client
    let client = TogetherClient::new(api_key)?;
    println!("✅ Connected to Together AI\n");

    // Single embedding
    println!("📝 Single embedding:");
    let request = EmbeddingRequest::new(
        "BAAI/bge-large-en-v1.5",
        "The quick brown fox jumps over the lazy dog",
    );

    let response = client.embeddings().create(request).await?;
    let embedding = response.first_embedding().expect("No embedding returned");
    println!("📊 Embedding dimensions: {}", embedding.len());
    println!(
        "📊 First 5 values: {:?}",
        &embedding[..5.min(embedding.len())]
    );
    println!("📊 Tokens used: {}", response.usage.total_tokens);

    // Multiple embeddings
    println!("\n📝 Multiple embeddings:");
    let texts = vec![
        "Machine learning is fascinating",
        "Deep learning is a subset of machine learning",
        "The weather is nice today",
    ];

    let request = EmbeddingRequest::new("BAAI/bge-large-en-v1.5", texts);
    let response = client.embeddings().create(request).await?;

    println!("📊 Generated {} embeddings", response.data.len());
    for (i, embedding) in response.data.iter().enumerate() {
        println!(
            "  Embedding {}: {} dimensions",
            i + 1,
            embedding.embedding.len()
        );
    }

    // Calculate similarity
    println!("\n📝 Cosine similarity between texts:");
    let embeddings: Vec<_> = response.all_embeddings();

    if embeddings.len() >= 2 {
        let sim_0_1 = cosine_similarity(embeddings[0], embeddings[1]);
        println!(
            "  'Machine learning...' <-> 'Deep learning...': {:.4}",
            sim_0_1
        );

        let sim_0_2 = cosine_similarity(embeddings[0], embeddings[2]);
        println!(
            "  'Machine learning...' <-> 'The weather...': {:.4}",
            sim_0_2
        );

        println!("\n  💡 Higher similarity = more semantically related");
    }

    // List embedding models
    println!("\n📝 Available embedding models:");
    let models = client.models().list().await?;
    let embedding_models = models.embedding_models();

    for model in embedding_models.iter().take(5) {
        println!(
            "  - {}: {}",
            model.id,
            model.description.as_deref().unwrap_or("No description")
        );
    }

    println!("\n✨ Done!");
    Ok(())
}
