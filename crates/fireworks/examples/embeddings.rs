//! Embeddings example for Fireworks AI.

use fireworks_ai::{
    FireworksClient,
    embeddings::{EmbeddingRequest, cosine_similarity},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get API key from environment
    let api_key = std::env::var("FIREWORKS_API_KEY")
        .expect("FIREWORKS_API_KEY environment variable must be set");

    // Create client
    let client = FireworksClient::new(api_key)?;
    println!("Created Fireworks client\n");

    // Single text embedding
    let request = EmbeddingRequest::new(
        "accounts/fireworks/models/nomic-embed-text-v1-5",
        "The quick brown fox jumps over the lazy dog",
    );

    println!("Generating embedding for single text...");
    let response = client.embeddings().create(request).await?;

    let embedding = response.first_embedding().expect("Expected an embedding");
    println!("Model: {}", response.model);
    println!("Embedding dimensions: {}", embedding.len());
    println!("Usage: {} tokens", response.usage.total_tokens);

    // Multiple texts embedding
    println!("\n--- Multiple Texts ---");
    let texts = vec![
        "Machine learning is fascinating",
        "Deep learning transforms AI",
        "The weather is nice today",
    ];

    let request = EmbeddingRequest::new("accounts/fireworks/models/nomic-embed-text-v1-5", texts);

    let response = client.embeddings().create(request).await?;
    println!("Generated {} embeddings", response.data.len());

    for (i, emb) in response.data.iter().enumerate() {
        println!("Embedding {}: {} dimensions", i + 1, emb.embedding.len());
    }

    // Cosine similarity example
    println!("\n--- Cosine Similarity ---");
    let texts = vec![
        "Artificial intelligence and machine learning",
        "AI and ML technologies",
        "Cooking recipes and food",
    ];

    let request = EmbeddingRequest::new("accounts/fireworks/models/nomic-embed-text-v1-5", texts);

    let response = client.embeddings().create(request).await?;
    let embeddings: Vec<_> = response.all_embeddings();

    if embeddings.len() >= 3 {
        let sim_0_1 = cosine_similarity(embeddings[0], embeddings[1]);
        let sim_0_2 = cosine_similarity(embeddings[0], embeddings[2]);

        println!("Similarity between 'AI/ML' texts: {:.4}", sim_0_1);
        println!(
            "Similarity between 'AI' and 'Cooking' texts: {:.4}",
            sim_0_2
        );
        println!("(Higher similarity = more similar meaning)");
    }

    Ok(())
}
