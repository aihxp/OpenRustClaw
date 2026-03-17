//! Example: Fine-tuning with Together AI.
//!
//! Run with:
//! ```
//! TOGETHER_API_KEY=your-api-key cargo run --example fine_tuning --features fine-tuning
//! ```

use together_ai::{TogetherClient, fine_tuning::JobStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key =
        std::env::var("TOGETHER_API_KEY").expect("TOGETHER_API_KEY environment variable not set");

    // Create client
    let client = TogetherClient::new(api_key)?;
    println!("✅ Connected to Together AI\n");

    // List existing fine-tuning jobs
    println!("📝 Listing fine-tuning jobs:");
    let jobs = client.fine_tuning().list_jobs().await?;

    if jobs.data.is_empty() {
        println!("  No fine-tuning jobs found.");
    } else {
        for job in jobs.data.iter().take(5) {
            let status = JobStatus::parse(&job.status);
            let status_emoji = match status {
                JobStatus::Succeeded => "✅",
                JobStatus::Failed => "❌",
                JobStatus::Running => "🔄",
                JobStatus::Queued => "⏳",
                JobStatus::Cancelled => "🚫",
                _ => "❓",
            };
            println!(
                "  {} Job {} - Status: {} (Model: {})",
                status_emoji, job.id, job.status, job.model
            );

            if let Some(fine_tuned_model) = &job.fine_tuned_model {
                println!("     Fine-tuned model: {}", fine_tuned_model);
            }
        }
    }

    // List uploaded files
    println!("\n📝 Listing uploaded files:");
    let files = client.fine_tuning().list_files().await?;

    if files.data.is_empty() {
        println!("  No files found.");
        println!("\n💡 To upload a training file, create a .jsonl file with format:");
        println!(
            "   {{\"messages\": [{{\"role\": \"system\", \"content\": \"You are...\"}}, {{\"role\": \"user\", \"content\": \"...\"}}, {{\"role\": \"assistant\", \"content\": \"...\"}}]}}"
        );
        println!("\n   Then use:");
        println!(
            "   let file = client.fine_tuning().upload_file(\"training.jsonl\", \"fine-tune\").await?;"
        );
    } else {
        for file in files.data.iter().take(5) {
            println!(
                "  📄 {} - {} ({} bytes, {})",
                file.id, file.filename, file.bytes, file.purpose
            );
        }
    }

    // Example of creating a fine-tuning job (commented out - requires actual file)
    println!("\n📝 Example: Creating a fine-tuning job");
    println!("  (This is commented out - uncomment and provide a training file to run)");

    /*
    // Upload training file first
    println!("Uploading training file...");
    let file = client.fine_tuning()
        .upload_file("training_data.jsonl", "fine-tune")
        .await?;
    println!("  Uploaded: {} ({})", file.id, file.filename);

    // Create fine-tuning job
    println!("\nCreating fine-tuning job...");
    let request = FineTuneRequest::new("meta-llama/Llama-3-8b-chat-hf", &file.id)
        .n_epochs(3)
        .batch_size(4)
        .learning_rate(1e-5)
        .suffix("my-custom-assistant");

    let job = client.fine_tuning().create_job(request).await?;
    println!("  Created job: {}", job.id);
    println!("  Status: {}", job.status);

    // Poll for status
    println!("\nPolling for status...");
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        let job = client.fine_tuning().get_job(&job.id).await?;
        let status = JobStatus::parse(&job.status);

        println!("  Status: {}", job.status);

        if status.is_terminal() {
            if let Some(model) = &job.fine_tuned_model {
                println!("  ✅ Fine-tuned model: {}", model);
            }
            break;
        }
    }
    */

    // Show job events example
    if !jobs.data.is_empty() {
        let job_id = &jobs.data[0].id;
        println!("\n📝 Events for job {}:", job_id);

        match client.fine_tuning().list_events(job_id).await {
            Ok(events) => {
                for event in events.data.iter().take(5) {
                    let level_emoji = match event.level.as_str() {
                        "info" => "ℹ️",
                        "warning" => "⚠️",
                        "error" => "❌",
                        _ => "📝",
                    };
                    println!("  {} {}", level_emoji, event.message);
                }
            }
            Err(e) => {
                println!("  Could not fetch events: {}", e);
            }
        }
    }

    println!("\n✨ Done!");
    Ok(())
}
