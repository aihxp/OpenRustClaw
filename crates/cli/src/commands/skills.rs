use anyhow::Result;

pub async fn list() -> Result<()> {
    println!("Listing installed skills...");
    println!("TODO: Read skills directory, display installed skills");
    Ok(())
}

pub async fn install(name: &str) -> Result<()> {
    println!("Installing skill: {}", name);
    println!("TODO: Download skill from marketplace, verify signature, install");
    Ok(())
}

pub async fn verify(name: &str) -> Result<()> {
    println!("Verifying skill: {}", name);
    println!("TODO: Check Ed25519 signature against trusted keys");
    Ok(())
}
