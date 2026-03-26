use std::io::Result;

fn main() -> Result<()> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "../../proto/collective/v1/core.proto",
                "../../proto/mcp/v1/mcp.proto",
                "../../proto/livingdocs/v1/review.proto",
                "../../proto/work/v1/work.proto",
                "../../proto/observability/v1/execution.proto",
                // Phase 1: Provider unification
                "../../proto/provider/v1/provider.proto",
                "../../proto/research/v1/research.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
