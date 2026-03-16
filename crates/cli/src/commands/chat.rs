use anyhow::Result;

pub async fn run(provider: &str, model: Option<&str>) -> Result<()> {
    println!(
        "Interactive chat (provider: {}, model: {})",
        provider,
        model.unwrap_or("default")
    );
    println!("TODO: Initialize provider, start chat loop");
    Ok(())
}
