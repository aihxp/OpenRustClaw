//! Example: Rerank documents with Cohere

#[cfg(not(feature = "rerank"))]
fn main() {
    println!("This example requires the 'rerank' feature. Run with: cargo run --example rerank --features rerank");
}

#[cfg(feature = "rerank")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use cohere::{CohereClient, RerankRequest};

    // Get API key from environment
    let api_key = std::env::var("COHERE_API_KEY")
        .expect("COHERE_API_KEY environment variable must be set");

    // Create client
    let client = CohereClient::new(api_key)?;

    // Documents to rerank
    let documents = vec![
        "Carson City is the capital city of the American state of Nevada.".to_string(),
        "The Commonwealth of the Northern Mariana Islands is a group of islands in the Pacific Ocean. Its capital is Saipan.".to_string(),
        "Washington, D.C. (also known as simply Washington or D.C., and officially as the District of Columbia) is the capital of the United States.".to_string(),
        "Capital punishment (the death penalty) has existed in the United States since before the United States was a country.".to_string(),
        "Capitalization or capitalisation in English grammar is the use of a capital letter at the start of a word.".to_string(),
    ];

    // Create rerank request
    let request = RerankRequest::builder()
        .model("rerank-english-v3.0")
        .query("What is the capital of the United States?")
        .documents(documents)
        .top_n(3)
        .return_documents(true)
        .build();

    // Send request
    println!("Reranking documents...\n");
    let response = client.rerank().create(request).await?;

    // Print results
    println!("Top {} results:\n", response.len());

    for (i, result) in response.results.iter().enumerate() {
        println!("Rank {}:", i + 1);
        println!("  Original Index: {}", result.index);
        println!("  Relevance Score: {:.4}", result.relevance_score);
        if let Some(doc) = &result.document {
            println!("  Document: {}", doc.text);
        }
        println!();
    }

    Ok(())
}
