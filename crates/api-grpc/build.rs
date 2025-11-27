fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create generated directory if it doesn't exist
    let out_dir = "src/generated";
    std::fs::create_dir_all(out_dir)?;

    // Configure tonic-build to generate Rust code from proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(out_dir)
        .compile(
            &[
                "proto/benchmark.proto",
                "proto/submission.proto",
                "proto/leaderboard.proto",
                "proto/governance.proto",
                "proto/user.proto",
            ],
            &["proto"],
        )?;

    // Re-run build script if proto files change
    println!("cargo:rerun-if-changed=proto/benchmark.proto");
    println!("cargo:rerun-if-changed=proto/submission.proto");
    println!("cargo:rerun-if-changed=proto/leaderboard.proto");
    println!("cargo:rerun-if-changed=proto/governance.proto");
    println!("cargo:rerun-if-changed=proto/user.proto");

    Ok(())
}
