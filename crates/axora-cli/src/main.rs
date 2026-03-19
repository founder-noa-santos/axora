use anyhow::Context;
use axora_agents::ProviderKind;
use axora_core::{init_tracing, RuntimeBootstrap, RuntimeBootstrapOptions};
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "axora")]
#[command(about = "Batteries-included AXORA CLI")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Execute a mission against the current workspace.
    Do {
        /// Natural-language mission for AXORA.
        mission: String,
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Provider backend to use.
        #[arg(long, value_enum, default_value = "anthropic")]
        provider: ProviderArg,
        /// Model identifier to request.
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ProviderArg {
    Anthropic,
    Openai,
}

impl From<ProviderArg> for ProviderKind {
    fn from(value: ProviderArg) -> Self {
        match value {
            ProviderArg::Anthropic => ProviderKind::Anthropic,
            ProviderArg::Openai => ProviderKind::OpenAi,
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "axora=info");
    }
    init_tracing();

    match Cli::parse().command {
        Commands::Do {
            mission,
            workspace,
            provider,
            model,
        } => {
            let workspace_root = workspace
                .unwrap_or(std::env::current_dir().context("failed to determine current directory")?);
            let provider_kind: ProviderKind = provider.into();
            let model = model.unwrap_or_else(|| match provider_kind {
                ProviderKind::Anthropic => "claude-sonnet-4-5".to_string(),
                ProviderKind::OpenAi => "gpt-5.4".to_string(),
            });
            let result = RuntimeBootstrap::run_mission(
                RuntimeBootstrapOptions {
                    workspace_root,
                    provider: provider_kind,
                    model,
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
