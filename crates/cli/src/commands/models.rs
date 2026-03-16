use anyhow::Result;

pub async fn list() -> Result<()> {
    println!("Listing available models...");
    println!("TODO: Query providers for available models");
    Ok(())
}

pub async fn info(name: &str) -> Result<()> {
    println!("Showing info for model: {}", name);
    println!("TODO: Fetch model details from provider");
    Ok(())
}
