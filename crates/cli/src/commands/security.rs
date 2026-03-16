use anyhow::Result;

pub async fn audit() -> Result<()> {
    println!("Running security audit...");
    println!("TODO: Check skill signatures, sandbox policies, API key exposure");
    Ok(())
}

pub async fn generate_keys() -> Result<()> {
    println!("Generating Ed25519 keypair for skill signing...");
    println!("TODO: Generate keypair, write to secure location");
    Ok(())
}
