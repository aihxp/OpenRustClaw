//! Example demonstrating citations and search features.
//!
//! Run with:
//! ```bash
//! PERPLEXITY_API_KEY=your_key cargo run --example citations
//! ```

use perplexity::{ChatRequest, PerplexityClient, SearchRecencyFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("PERPLEXITY_API_KEY")
        .expect("PERPLEXITY_API_KEY environment variable must be set");

    let client = PerplexityClient::new(api_key)?;

    // Example 1: Basic search with citations
    println!("=== Example 1: Basic Search with Citations ===\n");

    let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
        .user("What are the latest developments in artificial intelligence in 2024?")
        .temperature(0.7)
        .build();

    match client.chat().complete(request).await {
        Ok(response) => {
            println!("Response:\n{}\n", response.content().unwrap_or("No content"));

            if response.has_citations() {
                println!("Sources:");
                for citation in response.get_citations() {
                    println!(
                        "  [{}] {}{}",
                        citation.index,
                        citation
                            .title
                            .as_ref()
                            .map(|t| format!("{} - ", t))
                            .unwrap_or_default(),
                        citation.url
                    );
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 2: Search with recency filter
    println!("\n=== Example 2: Search with Recency Filter (Last Week) ===\n");

    let request = ChatRequest::builder("llama-3.1-sonar-large-128k-online")
        .user("What major tech announcements happened recently?")
        .search_recency_filter(SearchRecencyFilter::Week)
        .return_related_questions(true)
        .build();

    match client.chat().complete(request).await {
        Ok(response) => {
            println!("Response:\n{}\n", response.content().unwrap_or("No content"));

            if response.has_citations() {
                println!("Sources:");
                for citation in response.get_citations() {
                    println!("  [{}] {}", citation.index, citation.url);
                }
            }

            if response.has_related_questions() {
                println!("\nRelated Questions:");
                for (i, question) in response.get_related_questions().iter().enumerate() {
                    println!("  {}. {}", i + 1, question.question);
                }
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    // Example 3: Comparing online vs chat models
    println!("\n=== Example 3: Chat Model (No Search) ===\n");

    let request = ChatRequest::builder("llama-3.1-sonar-large-128k-chat")
        .user("Explain the concept of ownership in Rust.")
        .build();

    match client.chat().complete(request).await {
        Ok(response) => {
            println!("Response:\n{}\n", response.content().unwrap_or("No content"));

            // Chat models don't have citations
            if !response.has_citations() {
                println!("(No citations - chat model without search)");
            }
        }
        Err(e) => eprintln!("Error: {}", e),
    }

    Ok(())
}
