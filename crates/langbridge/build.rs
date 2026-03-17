fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false) // We're the client
        .build_client(true)
        .compile_protos(
            &[
                "../../proto/orchestration.proto",
                "../../proto/tracing.proto",
            ],
            &["../../proto/"],
        )?;
    Ok(())
}
