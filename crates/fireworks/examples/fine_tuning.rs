//! Fine-tuning example for Fireworks AI.
//!
//! Note: This example demonstrates the API usage. In practice, you would need:
//! 1. A valid training file in JSONL format
//! 2. Sufficient credits in your Fireworks account

use fireworks_ai::fine_tuning::FineTuneRequest;
use fireworks_ai::FireworksClient;

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

    // List existing fine-tuning jobs
    println!("Listing existing fine-tuning jobs...");
    match client.fine_tuning().list_jobs().await {
        Ok(jobs) => {
            println!("Found {} jobs", jobs.data.len());
            for job in &jobs.data {
                println!(
                    "  - {}: {} (model: {}, status: {})",
                    job.id, 
                    job.fine_tuned_model.as_deref().unwrap_or("N/A"),
                    job.model,
                    job.status
                );
            }
        }
        Err(e) => {
            println!("Could not list jobs: {}", e);
        }
    }

    // List uploaded files
    println!("\nListing uploaded files...");
    match client.fine_tuning().list_files().await {
        Ok(files) => {
            println!("Found {} files", files.data.len());
            for file in &files.data {
                println!("  - {}: {} ({} bytes, purpose: {})",
                    file.id,
                    file.filename,
                    file.bytes,
                    file.purpose
                );
            }
        }
        Err(e) => {
            println!("Could not list files: {}", e);
        }
    }

    // Example: Create a fine-tuning job (commented out - requires actual file)
    /*
    println!("\nCreating fine-tuning job...");
    let request = FineTuneRequest::new(
        "accounts/fireworks/models/llama-v3p1-8b-instruct",
        "file-your-training-file-id"
    )
        .validation_file("file-your-validation-file-id")
        .n_epochs(3)
        .batch_size(4)
        .learning_rate(1e-5)
        .suffix("my-custom-model")
        .lora(true);

    let job = client.fine_tuning().create_job(request).await?;
    println!("Created job: {}", job.id);
    println!("Status: {}", job.status);
    */

    // Example: Get job details (commented out - requires actual job ID)
    /*
    let job_id = "ft-job-id";
    let job = client.fine_tuning().get_job(job_id).await?;
    println!("Job {} status: {}", job.id, job.status);
    
    // List events
    let events = client.fine_tuning().list_events(job_id).await?;
    println!("Events:");
    for event in &events.data {
        println!("  [{}] {}: {}", 
            chrono::DateTime::from_timestamp(event.created_at, 0)
                .map(|d| d.to_rfc3339())
                .unwrap_or_default(),
            event.level,
            event.message
        );
    }
    */

    println!("\nExample completed!");
    println!("To actually run fine-tuning:");
    println!("1. Prepare a JSONL training file");
    println!("2. Upload it using client.fine_tuning().upload_file()");
    println!("3. Create a job with the file ID");

    Ok(())
}
