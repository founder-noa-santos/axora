use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["../../proto/collective/v1/core.proto"], &["../../proto"])?;
    Ok(())
}
