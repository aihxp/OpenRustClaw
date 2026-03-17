//! Example: Prometheus metrics from vLLM

use vllm::VllmClient;

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

    // Get raw metrics
    println!("Fetching Prometheus metrics...\n");

    let raw_metrics = client.metrics().get().await?;

    // Print raw metrics (first 2000 chars)
    println!("=== Raw Metrics (first 2000 chars) ===");
    if raw_metrics.len() > 2000 {
        println!("{}...", &raw_metrics[..2000]);
        println!("\n... (truncated, total length: {} chars)", raw_metrics.len());
    } else {
        println!("{}", raw_metrics);
    }
    println!();

    // Get parsed metrics
    let metrics = client.metrics().get_parsed().await?;

    println!("=== Parsed Metrics ===");
    println!("{}", metrics);

    // Additional details
    println!("\n=== Additional Details ===");

    if let Some(ttft) = metrics.avg_time_to_first_token() {
        println!("Time to First Token (TTFT): {:.3} ms", ttft * 1000.0);
    }

    if let Some(tpot) = metrics.avg_time_per_output_token() {
        println!("Time Per Output Token (TPOT): {:.3} ms", tpot * 1000.0);
    }

    if let Some(latency) = metrics.avg_inference_latency() {
        println!("Average Inference Latency: {:.3} s", latency);
    }

    if let Some(total) = metrics.total_tokens() {
        println!("Total Tokens Processed: {}", total);
    }

    if let Some(gpu_usage) = metrics.gpu_cache_usage_perc {
        println!("GPU Cache Usage: {:.1}%", gpu_usage * 100.0);
    }

    if let Some(cpu_usage) = metrics.cpu_cache_usage_perc {
        println!("CPU Cache Usage: {:.1}%", cpu_usage * 100.0);
    }

    if let Some(preemptions) = metrics.num_preemption_total {
        println!("Total Preemptions: {}", preemptions);
    }

    Ok(())
}
