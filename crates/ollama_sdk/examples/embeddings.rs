//! Example: Generate embeddings with Ollama
//!
//! Run with: cargo run --example embeddings
//!
//! Note: Requires an embedding model like nomic-embed-text:
//!   ollama pull nomic-embed-text

use ollama_sdk::OllamaClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for debug output
    tracing_subscriber::fmt::init();

    // Create client with default local Ollama instance
    let client = OllamaClient::new("http://localhost:11434");

    // Single text embedding
    println!("=== Single Text Embedding ===\n");
    
    let text = "The quick brown fox jumps over the lazy dog";
    println!("Text: {}\n", text);

    match client.embeddings().generate("nomic-embed-text", text).await {
        Ok(response) => {
            println!("Embedding dimensions: {}", response.embedding.len());
            println!("First 5 values: {:?}", &response.embedding[..5.min(response.embedding.len())]);
            
            // Calculate magnitude
            let magnitude: f32 = response.embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
            println!("Vector magnitude: {:.4}", magnitude);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!("\nMake sure you have pulled the nomic-embed-text model:");
            eprintln!("  ollama pull nomic-embed-text");
            std::process::exit(1);
        }
    }

    // Batch embedding
    println!("\n=== Batch Embeddings ===\n");
    
    let texts = vec![
        "Rust is a systems programming language",
        "Python is great for data science",
        "JavaScript runs in browsers",
        "Rust focuses on safety and performance",
    ];

    match client.embeddings().generate_batch("nomic-embed-text", texts.clone()).await {
        Ok(embeddings) => {
            println!("Generated {} embeddings\n", embeddings.len());

            // Compute cosine similarities between embeddings
            for i in 0..embeddings.len() {
                for j in (i + 1)..embeddings.len() {
                    let sim = cosine_similarity(&embeddings[i].1, &embeddings[j].1);
                    println!(
                        "Similarity between \"{}\" and \"{}\": {:.4}",
                        truncate(&texts[i], 30),
                        truncate(&texts[j], 30),
                        sim
                    );
                }
            }

            // Find most similar pair
            let mut max_sim = -1.0;
            let mut max_pair = (0, 0);
            for i in 0..embeddings.len() {
                for j in (i + 1)..embeddings.len() {
                    let sim = cosine_similarity(&embeddings[i].1, &embeddings[j].1);
                    if sim > max_sim {
                        max_sim = sim;
                        max_pair = (i, j);
                    }
                }
            }
            println!("\nMost similar pair:");
            println!("  \"{}\"", texts[max_pair.0]);
            println!("  \"{}\"", texts[max_pair.1]);
            println!("  Similarity: {:.4}", max_sim);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    // Embeddings with custom options
    println!("\n=== Custom Options ===\n");
    
    use ollama_sdk::Options;
    
    let options = Options::builder()
        .num_thread(4)
        .build();

    match client.embeddings()
        .generate_with_options("nomic-embed-text", "Hello, world!", options)
        .await {
        Ok(response) => {
            println!("Generated embedding with custom thread settings");
            println!("Dimensions: {}", response.embedding.len());
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }

    Ok(())
}

/// Calculate cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// Truncate a string to max length
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}
