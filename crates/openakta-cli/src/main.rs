mod doc;

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
    /// Initialize AI-optimized project documentation.
    Doc {
        #[command(subcommand)]
        command: DocCommands,
    },
}

#[derive(Subcommand, Debug)]
enum DocCommands {
    /// Scaffold the standard akta-docs structure and config.
    Init {
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Explicit project name written into .akta-config.yaml.
        #[arg(long)]
        project_name: Option<String>,
        /// Allow reuse of a non-empty akta-docs directory.
        #[arg(long)]
        allow_non_empty: bool,
        /// Replace managed scaffold files that already exist.
        #[arg(long)]
        overwrite: bool,
    },
    /// Lint akta-docs markdown using strict GEO rules.
    Lint {
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Optional files or directories to lint. Defaults to `akta-docs/`.
        targets: Vec<PathBuf>,
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
        Commands::Doc { command } => match command {
            DocCommands::Init {
                workspace,
                project_name,
                allow_non_empty,
                overwrite,
            } => {
                let workspace_root = workspace.unwrap_or(
                    std::env::current_dir().context("failed to determine current directory")?,
                );

                let report = doc::scaffold::run_doc_init(doc::scaffold::DocInitOptions {
                    workspace_root,
                    allow_non_empty,
                    overwrite,
                    project_name,
                })
                .await?;

                println!("Initialized {}", report.docs_root.display());
                println!(
                    "Created {} documentation directories",
                    report.created_directories.len()
                );
                if !report.overwritten_files.is_empty() {
                    println!("Overwrote {} managed files", report.overwritten_files.len());
                }
                for (template_id, source) in report.template_sources {
                    println!("template:{template_id} source:{}", source.as_str());
                }
                for warning in report.warnings {
                    eprintln!("warning:{warning}");
                }
            }
            DocCommands::Lint { workspace, targets } => {
                let workspace_root = workspace.unwrap_or(
                    std::env::current_dir().context("failed to determine current directory")?,
                );

                let result = doc::lint::run_doc_lint(doc::lint::DocLintOptions {
                    workspace_root: workspace_root.clone(),
                    targets,
                })?;

                print!("{}", doc::lint::render_doc_lint(&result, &workspace_root));
                if result.summary.error_count > 0 {
                    std::process::exit(1);
                }
            }
        },
    }

    Ok(())
}
