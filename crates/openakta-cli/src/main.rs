mod auth;
mod doc;

use anyhow::Context;
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use openakta_agents::{FallbackPolicy, MessageSurface, ProviderInstanceId, ResponsePreference};
use openakta_core::{
    init_tracing, ControlPlaneRuntime, MessageRequest, RuntimeBootstrap, RuntimeBootstrapOptions,
};
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
    /// Authenticate the CLI with Clerk.
    Login {
        /// Print the authorization URL instead of opening the browser.
        #[arg(long)]
        no_browser: bool,
    },
    /// Clear locally stored credentials.
    Logout,
    /// Inspect and manage CLI authentication state.
    Auth {
        #[command(subcommand)]
        subcommand: AuthCommands,
    },
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
        /// Disable remote API usage and require local execution only.
        #[arg(long)]
        no_auth: bool,
    },
    /// Ask the configured model directly, optionally with lightweight code context.
    Ask {
        /// Natural-language prompt.
        prompt: String,
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Local provider instance id to use.
        #[arg(long)]
        local_instance: Option<String>,
        /// Model identifier to request.
        #[arg(long)]
        local_model: Option<String>,
        /// Override the local runtime base URL.
        #[arg(long)]
        local_base_url: Option<String>,
        /// Disable repository inspection and send only the prompt.
        #[arg(long, action = ArgAction::SetTrue)]
        no_code: bool,
    },
    /// Inspect durable work-session state.
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },
    /// Initialize AI-optimized project documentation.
    Doc {
        #[command(subcommand)]
        command: DocCommands,
    },
}

#[derive(Subcommand, Debug)]
enum AuthCommands {
    Status,
    Whoami,
    Refresh,
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

#[derive(Subcommand, Debug)]
enum SessionCommands {
    /// Show a persisted work session and its task graph.
    Show {
        /// Work session id emitted by `openakta do` or `openakta ask`.
        session_id: String,
        /// Override the workspace root.
        #[arg(long)]
        workspace: Option<PathBuf>,
        /// Emit the full snapshot as JSON.
        #[arg(long, action = ArgAction::SetTrue)]
        json: bool,
        /// Include artifacts in the human-readable output.
        #[arg(long, action = ArgAction::SetTrue)]
        artifacts: bool,
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
    load_env_files();

    // Initialize sqlite-vec BEFORE any SQLite connections
    // This is the canonical, idempotent initialization with two-tier verification
    // End users do NOT need to install sqlite-vec manually - it's statically linked
    openakta_memory::ensure_sqlite_vec_ready()
        .context("sqlite-vec initialization failed - this is a product bug, not user error")?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "openakta=info");
    }
    init_tracing();

    match Cli::parse().command {
        Commands::Login { no_browser } => {
            let auth = auth::AuthManager::from_env()?;
            let whoami = auth.login(auth::login::LoginOptions { no_browser }).await?;
            println!("Logged in as {}", whoami.user_id);
            if let Some(org_id) = whoami.org_id {
                println!("Organization: {}", org_id);
            }
            if let Some(email) = whoami.email {
                println!("Email: {}", email);
            }
            println!("Expires at {}", whoami.expires_at);
        }
        Commands::Logout => {
            let auth = auth::AuthManager::from_env()?;
            auth.logout().await?;
            println!("Logged out");
        }
        Commands::Auth { subcommand } => {
            let auth = auth::AuthManager::from_env()?;
            match subcommand {
                AuthCommands::Status => {
                    let status = auth.status().await?;
                    if status.authenticated {
                        println!("authenticated");
                        if let Some(user_id) = status.user_id {
                            println!("user_id: {}", user_id);
                        }
                        if let Some(org_id) = status.org_id {
                            println!("org_id: {}", org_id);
                        }
                        if let Some(expires_at) = status.expires_at {
                            println!("expires_at: {}", expires_at);
                        }
                    } else {
                        println!("not authenticated");
                    }
                }
                AuthCommands::Whoami => {
                    let whoami = auth.whoami().await?;
                    println!("user_id: {}", whoami.user_id);
                    if let Some(org_id) = whoami.org_id {
                        println!("org_id: {}", org_id);
                    }
                    if let Some(email) = whoami.email {
                        println!("email: {}", email);
                    }
                    println!("expires_at: {}", whoami.expires_at);
                }
                AuthCommands::Refresh => {
                    let whoami = auth.refresh().await?;
                    println!("refreshed session for {}", whoami.user_id);
                    println!("expires_at: {}", whoami.expires_at);
                }
            }
        }
        Commands::Do {
            mission,
            workspace,
            provider,
            model,
            local_instance,
            local_model,
            local_base_url,
            fallback_policy,
            no_auth,
        } => {
            let workspace_root = workspace.unwrap_or(
                std::env::current_dir().context("failed to determine current directory")?,
            );
            let auth_provider = if no_auth {
                None
            } else {
                let auth = auth::AuthManager::from_env()?;
                auth.whoami()
                    .await
                    .context("authentication required; run `openakta login` first")?;
                Some(auth.auth_provider().await)
            };
            let result = RuntimeBootstrap::handle_message(
                RuntimeBootstrapOptions {
                    workspace_root: workspace_root.clone(),
                    cloud_instance: provider.map(ProviderInstanceId),
                    cloud_model: model,
                    local_instance: local_instance.map(ProviderInstanceId),
                    local_model,
                    local_base_url,
                    fallback_policy: fallback_policy.map(FallbackPolicy::from),
                    routing_enabled: None,
                    local_validation_retry_budget: None,
                    start_background_services: !no_auth,
                    remote_enabled: !no_auth,
                    auth_provider,
                },
                MessageRequest {
                    message: mission,
                    workspace_root,
                    surface: MessageSurface::CliDo,
                    response_preference: ResponsePreference::PreferMission,
                    allow_code_context: true,
                    side_effects_allowed: true,
                    remote_enabled: !no_auth,
                    workspace_context_override: None,
                },
            )
            .await?;
            eprintln!("work_session_id: {}", result.work_session_id);

            if result.success {
                println!("{}", result.output);
            } else {
                // Bug class A hardening: never emit empty stderr on mission failure
                if result.output.trim().is_empty() {
                    eprintln!("Mission '{}' failed (tasks_failed: {}, tasks_completed: {}, duration: {:.1}s)",
                        result.mission_id, result.tasks_failed, result.tasks_completed, result.duration.as_secs_f64());
                    eprintln!("No merged output was recorded. Enable debug logs (RUST_LOG=debug) for details.");
                } else {
                    eprintln!("{}", result.output);
                }
                eprintln!("Mission failed. See logs above for details.");
                std::process::exit(1);
            }
        }
        Commands::Ask {
            prompt,
            workspace,
            local_instance,
            local_model,
            local_base_url,
            no_code,
        } => {
            let workspace_root = workspace.unwrap_or(
                std::env::current_dir().context("failed to determine current directory")?,
            );
            let output = RuntimeBootstrap::handle_message(
                RuntimeBootstrapOptions {
                    workspace_root: workspace_root.clone(),
                    local_instance: local_instance.map(ProviderInstanceId),
                    local_model,
                    local_base_url,
                    start_background_services: false,
                    remote_enabled: false,
                    ..Default::default()
                },
                MessageRequest {
                    message: prompt,
                    workspace_root,
                    surface: MessageSurface::CliAsk,
                    response_preference: ResponsePreference::PreferDirectReply,
                    allow_code_context: !no_code,
                    side_effects_allowed: false,
                    remote_enabled: false,
                    workspace_context_override: None,
                },
            )
            .await?;
            eprintln!("work_session_id: {}", output.work_session_id);
            println!("{}", output.output);
        }
        Commands::Session { command } => match command {
            SessionCommands::Show {
                session_id,
                workspace,
                json,
                artifacts,
            } => {
                let workspace_root = workspace.unwrap_or(
                    std::env::current_dir().context("failed to determine current directory")?,
                );
                let runtime = ControlPlaneRuntime::open(&workspace_root)?;
                let snapshot = runtime
                    .snapshot_session(&session_id)?
                    .with_context(|| format!("work session not found: {session_id}"))?;

                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&snapshot)
                            .context("serialize work session snapshot")?
                    );
                } else {
                    print_work_session_snapshot(&snapshot, artifacts);
                }
            }
        },
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

fn print_work_session_snapshot(
    snapshot: &openakta_core::WorkSessionSnapshot,
    include_artifacts: bool,
) {
    println!("session_id: {}", snapshot.session.session_id);
    println!("status: {}", scalar(&snapshot.session.status));
    println!("mode: {}", scalar(&snapshot.session.admitted_mode));
    println!("task_type: {}", scalar(&snapshot.session.task_type));
    println!("risk: {}", scalar(&snapshot.session.risk));
    println!("workspace_root: {}", snapshot.session.workspace_root);
    println!("request: {}", snapshot.session.request_text);
    if let Some(mission_id) = snapshot.session.mission_id.as_deref() {
        println!("mission_id: {}", mission_id);
    }
    if let Some(trace_session_id) = snapshot.session.trace_session_id.as_deref() {
        println!("trace_session_id: {}", trace_session_id);
    }
    if let Some(error_message) = snapshot.session.error_message.as_deref() {
        println!("error: {}", error_message);
    }

    println!("tasks:");
    for task in &snapshot.tasks {
        println!(
            "- {} lane={} status={} deps={} title={}",
            task.task_id,
            scalar(&task.lane),
            scalar(&task.status),
            if task.depends_on_task_ids.is_empty() {
                "-".to_string()
            } else {
                task.depends_on_task_ids.join(",")
            },
            task.title
        );
    }

    if let Some(outcome_json) = snapshot.session.outcome_json.as_ref() {
        println!(
            "outcome: {}",
            serde_json::to_string_pretty(outcome_json).unwrap_or_else(|_| outcome_json.to_string())
        );
    }

    if include_artifacts {
        println!("artifacts:");
        for artifact in &snapshot.artifacts {
            println!(
                "- {} kind={} task={} requirements={}",
                artifact.artifact_id,
                artifact.artifact_kind,
                artifact.task_id.as_deref().unwrap_or("-"),
                if artifact.requirement_refs.is_empty() {
                    "-".to_string()
                } else {
                    artifact.requirement_refs.join(",")
                }
            );
        }
    }
}

fn scalar<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "\"unknown\"".to_string())
        .trim_matches('"')
        .to_string()
}

fn load_env_files() {
    let mut seen = std::collections::HashSet::new();
    let mut candidates = Vec::new();
    let mut current = std::env::current_dir().ok();

    while let Some(dir) = current {
        candidates.push(dir.join(".env.local"));
        candidates.push(dir.join(".env"));
        candidates.push(dir.join("openakta-api").join(".env.local"));
        candidates.push(dir.join("openakta-api").join(".env"));
        candidates.push(dir.join("openakta-web").join(".env.local"));
        candidates.push(dir.join("openakta-web").join(".env"));

        if let Some(parent) = dir.parent() {
            current = Some(parent.to_path_buf());
        } else {
            break;
        }
    }

    for path in candidates {
        if seen.insert(path.clone()) && path.exists() {
            let _ = dotenvy::from_path(&path);
        }
    }
}
