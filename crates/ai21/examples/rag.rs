//! Example: RAG with AI21 Contextual Answers

use ai21::{Ai21Client, ContextualAnswersRequest, Document};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("AI21_API_KEY")
        .expect("AI21_API_KEY environment variable must be set");

    // Create client
    let client = Ai21Client::new(api_key)?;

    // Create documents for context
    let documents = vec![
        Document::new("doc1", "The Eiffel Tower is a wrought-iron lattice tower on the Champ de Mars in Paris, France. It is named after the engineer Gustave Eiffel."),
        Document::new("doc2", "Paris is the capital and most populous city of France. It is located on the Seine River in the north of the country."),
        Document::new("doc3", "The Louvre is the world's most-visited museum and a historic monument in Paris, France. It is the home of the Mona Lisa."),
    ];

    // Create contextual answers request
    let request = ContextualAnswersRequest::new(
        "Where is the Eiffel Tower located and who is it named after?",
        documents,
    );

    // Send request
    println!("Sending RAG request to AI21...");
    let response = client.rag().contextual_answers(request).await?;

    // Print response
    println!("Answer: {}", response.answer);
    if let Some(confidence) = response.confidence() {
        println!("Confidence: {:.2}%", confidence * 100.0);
    }

    Ok(())
}
