//! Example: Embeddings with vLLM

use vllm::{EmbeddingRequest, VllmClient, embeddings::cosine_similarity};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create client
    let base_url = std::env::var("VLLM_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
    let client = VllmClient::new(&base_url)?;

    println!("Connected to vLLM at: {}", client.base_url());

    // Check health
    match client.health().await {
        Ok(()) => println!("✓ vLLM server is healthy\n"),
        Err(e) => {
            eprintln!("✗ Health check failed: {}", e);
            return Ok(());
        }
    }

    // Get the embedding model
    let model = std::env::var("VLLM_EMBEDDING_MODEL")
        .unwrap_or_else(|_| "BAAI/bge-large-en-v1.5".to_string());

    // Example sentences for embedding
    let sentences = vec![
        "The cat sits on the mat.",
        "A feline rests on the rug.",
        "The stock market crashed today.",
    ];

    println!("Generating embeddings using model: {}", model);
    println!("Sentences to embed:");
    for (i, sentence) in sentences.iter().enumerate() {
        println!("  {}. {}", i + 1, sentence);
    }
    println!();

    // Create embedding request
    let request = EmbeddingRequest::new(&model, sentences.clone());

    // Get embeddings
    let response = client.embeddings().create(request).await?;

    println!("Embeddings generated successfully!");
    println!("Model used: {}", response.model);
    println!("Prompt tokens: {}", response.usage.prompt_tokens);
    println!("Total tokens: {}", response.usage.total_tokens);
    println!();

    // Get all embeddings
    let embeddings: Vec<_> = response.all_embeddings();
    println!("Embedding dimensions: {}", embeddings[0].len());
    println!();

    // Calculate similarities
    println!("Cosine similarities:");
    for i in 0..sentences.len() {
        for j in (i + 1)..sentences.len() {
            let similarity = cosine_similarity(embeddings[i], embeddings[j]);
            println!(
                "  '{}' <-> '{}': {:.4}",
                sentences[i], sentences[j], similarity
            );
        }
    }

    // Expected: sentences 0 and 1 should have high similarity (both about cats)
    // Sentence 2 should have low similarity with both (about stock market)
    let sim_0_1 = cosine_similarity(embeddings[0], embeddings[1]);
    let sim_0_2 = cosine_similarity(embeddings[0], embeddings[2]);

    println!();
    if sim_0_1 > sim_0_2 {
        println!("✓ Cat sentences are more similar to each other than to stock market sentence");
    } else {
        println!("✗ Unexpected similarity pattern");
    }

    Ok(())
}
