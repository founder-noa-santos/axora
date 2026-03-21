use anyhow::Context;
use clap::{Parser, Subcommand, ValueEnum};
use openakta_agents::{FallbackPolicy, ProviderInstanceId};
use openakta_core::{init_tracing, RuntimeBootstrap, RuntimeBootstrapOptions};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "openakta")]
#[command(about = "Batteries-included OPENAKTA CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute a mission against the current workspace.
    Do {
        /// Natural-language mission for OPENAKTA.
        mission: String,
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Cloud provider instance id to use.
        #[arg(long)]
        provider: Option<String>,
        /// Model identifier to request.
        #[arg(long)]
        model: Option<String>,
        /// Local provider instance id to use.
        #[arg(long)]
        local_instance: Option<String>,
        /// Enable a local fast-path model.
        #[arg(long)]
        local_model: Option<String>,
        /// Override the local runtime base URL.
        #[arg(long)]
        local_base_url: Option<String>,
        /// Override the fallback policy.
        #[arg(long, value_enum)]
        fallback_policy: Option<FallbackPolicyArg>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum FallbackPolicyArg {
    Never,
    Explicit,
    Automatic,
}

impl From<FallbackPolicyArg> for FallbackPolicy {
    fn from(value: FallbackPolicyArg) -> Self {
        match value {
            FallbackPolicyArg::Never => FallbackPolicy::Never,
            FallbackPolicyArg::Explicit => FallbackPolicy::Explicit,
            FallbackPolicyArg::Automatic => FallbackPolicy::Automatic,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "openakta=info");
    }
    init_tracing();

    match Cli::parse().command {
        Commands::Do {
            mission,
            workspace,
            provider,
            model,
            local_instance,
            local_model,
            local_base_url,
            fallback_policy,
        } => {
            let workspace_root = workspace.unwrap_or(
                std::env::current_dir().context("failed to determine current directory")?,
            );
            let result = RuntimeBootstrap::run_mission(
                RuntimeBootstrapOptions {
                    workspace_root,
                    cloud_instance: provider.map(ProviderInstanceId),
                    cloud_model: model,
                    local_instance: local_instance.map(ProviderInstanceId),
                    local_model,
                    local_base_url,
                    fallback_policy: fallback_policy.map(FallbackPolicy::from),
                    routing_enabled: None,
                    local_validation_retry_budget: None,
                    start_background_services: true,
                },
                &mission,
            )
            .await?;

            if result.success {
                println!("{}", result.output);
            } else {
                eprintln!("{}", result.output);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}
