//! Fine-tuning API example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DEPLOYMENT=your-deployment \
//! TRAINING_FILE_ID=your-training-file-id \
//! cargo run --example fine_tuning --features fine-tuning
//! ```

#[cfg(not(feature = "fine-tuning"))]
fn main() {
    println!("This example requires the 'fine-tuning' feature.");
    println!("Run with: cargo run --example fine_tuning --features fine-tuning");
}

#[cfg(feature = "fine-tuning")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use azure_openai::{AzureOpenAIClient, fine_tuning::*};
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
        .expect("AZURE_OPENAI_DEPLOYMENT environment variable not set");

    // Create the client
    let client = AzureOpenAIClient::new(
        &resource_name,
        &deployment_name,
        api_key.clone(),
    )?;

    println!("Azure OpenAI Fine-tuning API Example\n");

    // List existing fine-tuning jobs
    println!("1. Listing existing fine-tuning jobs...");
    let jobs = client.fine_tuning().list(Some(10)).await?;
    println!("   Found {} jobs", jobs.data.len());
    
    for job in &jobs.data {
        println!(
            "   - {}: {:?} (model: {})",
            job.id,
            job.status,
            job.model
        );
    }

    // Create a fine-tuning job if training file is provided
    if let Ok(training_file_id) = std::env::var("TRAINING_FILE_ID") {
        println!("\n2. Creating Fine-tuning Job...");
        
        let hyperparameters = Hyperparameters::auto()
            .n_epochs(3)
            .batch_size(4);

        let job_request = FineTuningJobRequest::new(&training_file_id)
            .model("gpt-35-turbo-0613")
            .hyperparameters(hyperparameters)
            .suffix("custom-model")
            .seed(42);

        let job = client.fine_tuning().create(job_request).await?;
        println!("   Job ID: {}", job.id);
        println!("   Model: {}", job.model);
        println!("   Status: {:?}", job.status);
        println!("   Training file: {}", job.training_file);

        // Poll for status
        println!("\n3. Polling job status...");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            
            let current_job = client.fine_tuning().retrieve(&job.id).await?;
            println!("   Status: {:?}", current_job.status);
            
            if current_job.status.is_terminal() {
                if let Some(model) = &current_job.fine_tuned_model {
                    println!("   Fine-tuned model: {}", model);
                }
                if let Some(error) = &current_job.error {
                    println!("   Error: {} - {}", error.code, error.message);
                }
                break;
            }
        }

        // List events
        println!("\n4. Listing job events...");
        let events = client.fine_tuning().list_events(&job.id).await?;
        println!("   Found {} events", events.data.len());
        
        for event in events.data.iter().take(5) {
            println!(
                "   [{}] {} - {}",
                event.created_at,
                event.level,
                event.message
            );
        }

        // List checkpoints
        println!("\n5. Listing checkpoints...");
        let checkpoints = client.fine_tuning().list_checkpoints(&job.id).await?;
        println!("   Found {} checkpoints", checkpoints.data.len());
        
        for checkpoint in &checkpoints.data {
            println!(
                "   - Step {}: {} (train_loss: {:.4})",
                checkpoint.step_number,
                checkpoint.fine_tuned_model_checkpoint,
                checkpoint.metrics.train_loss
            );
        }
    } else {
        println!("\n2. To create a fine-tuning job, set TRAINING_FILE_ID environment variable");
        println!("   First, upload a training file using the Files API:");
        println!("   File format should be JSONL with messages array per line:");
        println!("   {{\"messages\": [");
        println!("       {{\"role\": \"system\", \"content\": \"You are...\"}},");
        println!("       {{\"role\": \"user\", \"content\": \"Input...\"}},");
        println!("       {{\"role\": \"assistant\", \"content\": \"Output...\"}}");
        println!("   ]}}");
    }

    println!("\nDone!");
    Ok(())
}
