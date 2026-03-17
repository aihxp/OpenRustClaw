//! Assistants API example for Azure OpenAI.
//!
//! Run with:
//! ```bash
//! AZURE_OPENAI_API_KEY=your-key \
//! AZURE_OPENAI_RESOURCE=your-resource \
//! AZURE_OPENAI_DEPLOYMENT=your-deployment \
//! cargo run --example assistants --features assistants
//! ```

#[cfg(not(feature = "assistants"))]
fn main() {
    println!("This example requires the 'assistants' feature.");
    println!("Run with: cargo run --example assistants --features assistants");
}

#[cfg(feature = "assistants")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use azure_openai::{AzureOpenAIClient, assistants::*};
    // Load configuration from environment
    let api_key = std::env::var("AZURE_OPENAI_API_KEY")
        .expect("AZURE_OPENAI_API_KEY environment variable not set");
    let resource_name = std::env::var("AZURE_OPENAI_RESOURCE")
        .expect("AZURE_OPENAI_RESOURCE environment variable not set");
    let deployment_name = std::env::var("AZURE_OPENAI_DEPLOYMENT")
        .expect("AZURE_OPENAI_DEPLOYMENT environment variable not set");

    // Create the client
    let client = AzureOpenAIClient::new(&resource_name, &deployment_name, api_key.clone())?;

    println!("Azure OpenAI Assistants API Example\n");

    // Create an assistant
    println!("1. Creating Assistant...");
    let assistant_request = AssistantRequest::new("gpt-4")
        .name("Math Tutor")
        .instructions("You are a helpful math tutor. Explain concepts clearly and step by step.")
        .enable_code_interpreter();

    let assistant = client.assistants().create(assistant_request).await?;
    println!("   Assistant ID: {}", assistant.id);
    println!("   Name: {}", assistant.name.as_deref().unwrap_or("N/A"));
    println!("   Model: {}", assistant.model);

    // Create a thread
    println!("\n2. Creating Thread...");
    let thread = client.assistants().create_thread(None).await?;
    println!("   Thread ID: {}", thread.id);

    // Add a message to the thread
    println!("\n3. Adding Message...");
    let message_request =
        ThreadMessageRequest::user("I need to solve the equation 3x + 11 = 14. Can you help me?");
    let message = client
        .assistants()
        .create_message(&thread.id, message_request)
        .await?;
    println!("   Message ID: {}", message.id);
    println!("   Role: {}", message.role);

    // Create a run
    println!("\n4. Creating Run...");
    let run_request = RunRequest::new(&assistant.id);
    let run = client
        .assistants()
        .create_run(&thread.id, run_request)
        .await?;
    println!("   Run ID: {}", run.id);
    println!("   Status: {:?}", run.status);

    // Poll for completion
    println!("\n5. Waiting for completion...");
    let mut current_run = run;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        current_run = client
            .assistants()
            .retrieve_run(&thread.id, &current_run.id)
            .await?;

        println!("   Status: {:?}", current_run.status);

        match current_run.status {
            RunStatus::Completed => break,
            RunStatus::Failed | RunStatus::Cancelled | RunStatus::Expired => {
                eprintln!("   Run failed!");
                break;
            }
            _ => continue,
        }
    }

    // Retrieve messages
    println!("\n6. Retrieving Messages...");
    let messages = client.assistants().list_messages(&thread.id).await?;

    for msg in messages.data.iter().rev() {
        println!("\n   {}: ", msg.role);
        for content in &msg.content {
            match content {
                MessageContent::Text { text } => {
                    println!("{}", text.value);
                }
                MessageContent::ImageFile { image_file } => {
                    println!("[Image: {}]", image_file.file_id);
                }
            }
        }
    }

    // Clean up
    println!("\n7. Cleaning up...");
    client.assistants().delete_thread(&thread.id).await?;
    println!("   Thread deleted");

    client.assistants().delete(&assistant.id).await?;
    println!("   Assistant deleted");

    println!("\nDone!");
    Ok(())
}
