//! Example: Tokenization with vLLM

use vllm::VllmClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create client
    let base_url =
        std::env::var("VLLM_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
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

    // Get the model
    let model = std::env::var("VLLM_MODEL")
        .unwrap_or_else(|_| "meta-llama/Meta-Llama-3-8B-Instruct".to_string());

    // Example text to tokenize
    let text = "Hello, world! This is a test of the vLLM tokenization API.";

    println!("Model: {}", model);
    println!("Text: \"{}\"\n", text);

    // Tokenize
    println!("Tokenizing...");
    let tokens = client.tokenize().tokenize(&model, text).await?;

    println!("Token count: {}", tokens.len());
    println!("Token IDs: {:?}\n", tokens);

    // Detokenize
    println!("Detokenizing back to text...");
    let recovered_text = client.tokenize().detokenize(&model, &tokens).await?;

    println!("Recovered text: \"{}\"\n", recovered_text);

    // Check if round-trip is successful
    if recovered_text.trim() == text.trim() {
        println!("✓ Round-trip successful!");
    } else {
        println!("Note: Round-trip may differ due to tokenizer normalization");
        println!("  Original:  \"{}\"", text);
        println!("  Recovered: \"{}\"", recovered_text);
    }

    // Count tokens only (convenience method)
    println!();
    let count = client.tokenize().count_tokens(&model, text).await?;
    println!("Token count (via count_tokens): {}", count);

    Ok(())
}
