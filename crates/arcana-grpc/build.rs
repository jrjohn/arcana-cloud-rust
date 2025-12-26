//! Build script for compiling protobuf definitions.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile proto files to OUT_DIR
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../../proto/common.proto",
                "../../proto/user_service.proto",
                "../../proto/auth_service.proto",
                "../../proto/health.proto",
                "../../proto/repository_service.proto",
            ],
            &["../../proto"],
        )?;

    // Rerun if proto files change
    println!("cargo:rerun-if-changed=../../proto/");

    Ok(())
}
