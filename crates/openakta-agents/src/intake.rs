use crate::task::TaskType;
use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

const MAX_CONTEXT_FILE_BYTES: u64 = 64 * 1024;
const MAX_CONTEXT_FILE_CHARS: usize = 4_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MessageSurface {
    CliAsk,
    CliDo,
    Daemon,
    Desktop,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ResponsePreference {
    Auto,
    PreferDirectReply,
    PreferMission,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MessageExecutionMode {
    DirectReply,
    DirectAction,
    SingleAgent,
    MultiStep,
    Delegated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct TaskTargetHints {
    pub target_files: Vec<String>,
    pub target_symbols: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RetrievalPlan {
    pub repo_context_requested: bool,
    pub max_hits: usize,
    pub workspace_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DecompositionBudget {
    pub max_tasks: usize,
    pub max_parallelism: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DelegationBudget {
    pub max_agents: usize,
    pub max_depth: usize,
    pub allow_delegation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MissionDecision {
    pub mode: MessageExecutionMode,
    pub retrieval_plan: RetrievalPlan,
    pub target_hints: TaskTargetHints,
    pub decomposition_budget: DecompositionBudget,
    pub delegation_budget: DelegationBudget,
    pub task_type: TaskType,
    pub risk: RiskLevel,
}

#[derive(Debug, Clone)]
pub struct MissionGateRequest<'a> {
    pub message: &'a str,
    pub workspace_root: &'a Path,
    pub surface: MessageSurface,
    pub response_preference: ResponsePreference,
    pub allow_code_context: bool,
    pub side_effects_allowed: bool,
    pub workspace_context_override: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ComplexityAssessment {
    pub normalized_message: String,
    pub token_count: usize,
    pub is_question: bool,
    pub side_effects_requested: bool,
    pub repo_context_requested: bool,
    pub read_only_analysis_requested: bool,
    pub broad_scope: bool,
    pub dependency_count: usize,
    pub deliverable_count: usize,
    pub explicit_target_count: usize,
    pub risk: RiskLevel,
    pub trivial_reply_candidate: bool,
    pub direct_action_candidate: bool,
    pub target_hints: TaskTargetHints,
    pub workspace_context: Option<String>,
}

pub struct MissionGate;

impl MissionGate {
    pub fn analyze(request: &MissionGateRequest<'_>) -> anyhow::Result<MissionDecision> {
        let assessment = assess_message(request)?;
        Ok(decide_message(request, assessment))
    }
}

fn assess_message(request: &MissionGateRequest<'_>) -> anyhow::Result<ComplexityAssessment> {
    let normalized_message = request.message.trim().to_string();
    let target_hints = extract_target_hints(&normalized_message);
    let token_count = prompt_tokens(&normalized_message).len().max(
        normalized_message
            .split_whitespace()
            .filter(|segment| !segment.is_empty())
            .count(),
    );
    let lowered = normalized_message.to_ascii_lowercase();
    let is_question = normalized_message.ends_with('?')
        || lowered.starts_with("what ")
        || lowered.starts_with("why ")
        || lowered.starts_with("how ")
        || lowered.starts_with("where ")
        || lowered.starts_with("when ")
        || lowered.starts_with("who ");
    let side_effects_requested =
        request.side_effects_allowed && contains_any(&lowered, SIDE_EFFECT_KEYWORDS);
    let repo_context_requested = request.allow_code_context
        && (!target_hints.target_files.is_empty()
            || !target_hints.target_symbols.is_empty()
            || contains_any(&lowered, REPO_CONTEXT_KEYWORDS));
    let read_only_analysis_requested = repo_context_requested
        && !side_effects_requested
        && contains_any(&lowered, ANALYSIS_KEYWORDS)
        && !contains_any(&lowered, SIDE_EFFECT_KEYWORDS);
    let dependency_count = count_dependencies(&lowered);
    let deliverable_count = estimate_deliverables(&lowered);
    let explicit_target_count = target_hints.target_files.len() + target_hints.target_symbols.len();
    let broad_scope = explicit_target_count > 1
        || contains_any(&lowered, BROAD_SCOPE_KEYWORDS)
        || (deliverable_count > 1 && side_effects_requested);
    let trivial_reply_candidate =
        is_trivial_direct_reply(&lowered, token_count, side_effects_requested);
    let direct_action_candidate = side_effects_requested
        && !broad_scope
        && deliverable_count <= 1
        && dependency_count == 0
        && explicit_target_count <= 1;
    let risk = classify_risk(&lowered, broad_scope, side_effects_requested);
    let workspace_context = if let Some(context) = request.workspace_context_override.clone() {
        Some(context)
    } else if repo_context_requested {
        Some(build_workspace_context(
            request.workspace_root,
            &normalized_message,
            if explicit_target_count == 0 { 2 } else { 3 },
        )?)
    } else {
        None
    };

    Ok(ComplexityAssessment {
        normalized_message,
        token_count,
        is_question,
        side_effects_requested,
        repo_context_requested,
        read_only_analysis_requested,
        broad_scope,
        dependency_count,
        deliverable_count,
        explicit_target_count,
        risk,
        trivial_reply_candidate,
        direct_action_candidate,
        target_hints,
        workspace_context,
    })
}

fn decide_message(
    request: &MissionGateRequest<'_>,
    assessment: ComplexityAssessment,
) -> MissionDecision {
    let mode = if assessment.trivial_reply_candidate {
        MessageExecutionMode::DirectReply
    } else if assessment.read_only_analysis_requested {
        MessageExecutionMode::SingleAgent
    } else if !assessment.side_effects_requested
        && !assessment.broad_scope
        && assessment.dependency_count == 0
        && assessment.deliverable_count <= 1
        && (request.response_preference == ResponsePreference::PreferDirectReply
            || !assessment.repo_context_requested
            || assessment.explicit_target_count <= 1)
    {
        MessageExecutionMode::DirectReply
    } else if assessment.direct_action_candidate {
        MessageExecutionMode::DirectAction
    } else if assessment.side_effects_requested
        && (assessment.broad_scope
            || assessment.dependency_count > 0
            || assessment.deliverable_count > 1)
    {
        MessageExecutionMode::MultiStep
    } else if !assessment.side_effects_requested
        && contains_any(
            &assessment.normalized_message.to_ascii_lowercase(),
            EXPLICIT_MULTI_PHASE_KEYWORDS,
        )
    {
        MessageExecutionMode::MultiStep
    } else {
        MessageExecutionMode::SingleAgent
    };

    let task_type = if assessment.side_effects_requested
        && !assessment.target_hints.target_files.is_empty()
        && contains_any(
            &assessment.normalized_message.to_ascii_lowercase(),
            CODE_EDIT_KEYWORDS,
        ) {
        TaskType::CodeModification
    } else if assessment.read_only_analysis_requested
        || (assessment.repo_context_requested && !assessment.side_effects_requested)
    {
        TaskType::Retrieval
    } else {
        TaskType::General
    };

    let retrieval_plan = RetrievalPlan {
        repo_context_requested: assessment.repo_context_requested,
        max_hits: if assessment.read_only_analysis_requested {
            4
        } else if mode == MessageExecutionMode::DirectReply {
            2
        } else {
            4
        },
        workspace_context: assessment.workspace_context,
    };

    let decomposition_budget = match mode {
        MessageExecutionMode::DirectReply
        | MessageExecutionMode::DirectAction
        | MessageExecutionMode::SingleAgent => DecompositionBudget {
            max_tasks: 1,
            max_parallelism: 1,
        },
        MessageExecutionMode::MultiStep => DecompositionBudget {
            max_tasks: 3,
            max_parallelism: 2,
        },
        MessageExecutionMode::Delegated => DecompositionBudget {
            max_tasks: 4,
            max_parallelism: 2,
        },
    };

    let delegation_budget = DelegationBudget {
        max_agents: 0,
        max_depth: 0,
        allow_delegation: false,
    };

    MissionDecision {
        mode,
        retrieval_plan,
        target_hints: assessment.target_hints,
        decomposition_budget,
        delegation_budget,
        task_type,
        risk: assessment.risk,
    }
}

pub fn extract_target_hints(message: &str) -> TaskTargetHints {
    let mut target_files = Vec::new();
    let mut target_symbols = Vec::new();

    for token in message.split_whitespace() {
        let cleaned = token
            .trim_matches(|ch: char| matches!(ch, ',' | '.' | ';' | ':' | '(' | ')' | '"' | '\''))
            .to_string();
        if cleaned.is_empty() {
            continue;
        }
        if cleaned.contains("::") {
            target_symbols.push(cleaned.clone());
        }
        if cleaned.contains('/')
            || FILE_EXTENSIONS
                .iter()
                .any(|ext| cleaned.to_ascii_lowercase().ends_with(ext))
        {
            target_files.push(cleaned);
        }
    }

    target_files.sort();
    target_files.dedup();
    target_symbols.sort();
    target_symbols.dedup();

    TaskTargetHints {
        target_files,
        target_symbols,
    }
}

pub fn build_workspace_context(
    workspace_root: &Path,
    prompt: &str,
    max_context_files: usize,
) -> anyhow::Result<String> {
    let top_level = top_level_entries(workspace_root)?;
    let prompt_tokens = prompt_tokens(prompt);
    let mut relevant_files =
        if let Some(manifest_files) = manifest_bootstrap_files(workspace_root, prompt)? {
            manifest_files
        } else if is_repository_analysis_prompt(prompt) {
            high_signal_files(workspace_root, max_context_files)?
        } else {
            relevant_files(workspace_root, &prompt_tokens, max_context_files)?
        };
    if relevant_files.is_empty() {
        relevant_files = high_signal_files(workspace_root, max_context_files)?;
    }

    let mut sections = Vec::new();
    sections.push(format!("Workspace root: {}", workspace_root.display()));

    if !top_level.is_empty() {
        sections.push(format!("Top-level entries:\n{}", top_level.join("\n")));
    }

    if !relevant_files.is_empty() {
        let mut rendered = Vec::new();
        for path in relevant_files {
            let relative = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .display()
                .to_string();
            let content = std::fs::read_to_string(&path)?;
            let snippet = truncate_chars(&content, MAX_CONTEXT_FILE_CHARS);
            rendered.push(format!("File: {relative}\n{snippet}"));
        }
        sections.push(format!(
            "Relevant code context:\n{}",
            rendered.join("\n\n---\n\n")
        ));
    }

    Ok(sections.join("\n\n"))
}

fn top_level_entries(root: &Path) -> anyhow::Result<Vec<String>> {
    let mut entries = std::fs::read_dir(root)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| !ignored_name(name))
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

fn prompt_tokens(prompt: &str) -> HashSet<String> {
    prompt
        .split(|c: char| !c.is_ascii_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect()
}

fn relevant_files(
    root: &Path,
    tokens: &HashSet<String>,
    max_context_files: usize,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut scored = Vec::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_descend(entry))
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry
            .metadata()
            .map(|metadata| metadata.len() > MAX_CONTEXT_FILE_BYTES)
            .unwrap_or(true)
        {
            continue;
        }

        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);
        let path_text = relative.display().to_string().to_ascii_lowercase();
        let mut score = 0usize;

        for token in tokens {
            if path_text.contains(token) {
                score += 10;
            }
        }

        if score == 0 && tokens.is_empty() {
            score = 1;
        }

        if score == 0 {
            continue;
        }

        if std::fs::read_to_string(path).is_err() {
            continue;
        }

        scored.push((Reverse(score), relative.to_path_buf(), path.to_path_buf()));
    }

    scored.sort();
    Ok(scored
        .into_iter()
        .take(max_context_files)
        .map(|(_, _, absolute)| absolute)
        .collect())
}

fn high_signal_files(root: &Path, max_context_files: usize) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let candidates = [
        "Cargo.toml",
        "openakta.toml",
        "README.md",
        "crates/openakta-cli/src/main.rs",
        "crates/openakta-core/src/bootstrap.rs",
        "crates/openakta-agents/src/coordinator/v2.rs",
        "crates/openakta-agents/src/intake.rs",
    ];

    for candidate in candidates {
        let path = root.join(candidate);
        if path.is_file() {
            files.push(path);
        }
        if files.len() >= max_context_files {
            return Ok(files);
        }
    }

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|entry| should_descend(entry))
        .filter_map(|entry| entry.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        let relative = path.strip_prefix(root).unwrap_or(&path);
        let relative_text = relative.display().to_string();
        let is_anchor = relative_text.ends_with("src/main.rs")
            || relative_text.ends_with("src/lib.rs")
            || relative_text.ends_with("README.md");
        if !is_anchor || files.iter().any(|existing| existing == &path) {
            continue;
        }
        files.push(path);
        if files.len() >= max_context_files {
            break;
        }
    }

    Ok(files)
}

fn manifest_bootstrap_files(root: &Path, prompt: &str) -> anyhow::Result<Option<Vec<PathBuf>>> {
    let lowered = prompt.to_ascii_lowercase();
    let wants_package_json = lowered.contains("package.json")
        || lowered.contains("manifest")
        || lowered.contains("workspace");
    if !wants_package_json {
        return Ok(None);
    }

    let wants_root =
        lowered.contains("root") || lowered.contains("raiz") || lowered.contains("workspace");
    let wants_apps = lowered.contains("apps/")
        || lowered.contains(" apps ")
        || lowered.contains("dentro de apps")
        || lowered.contains("inside apps");
    let wants_packages = lowered.contains("packages/")
        || lowered.contains(" packages ")
        || lowered.contains("dentro de packages")
        || lowered.contains("inside packages");

    let mut files = Vec::new();
    if wants_root {
        let path = root.join("package.json");
        if path.is_file() {
            files.push(path);
        }
    }
    if wants_apps {
        files.extend(package_json_children(root.join("apps"))?);
    }
    if wants_packages {
        files.extend(package_json_children(root.join("packages"))?);
    }
    files.sort();
    files.dedup();

    if files.is_empty() {
        Ok(None)
    } else {
        Ok(Some(files.into_iter().take(8).collect()))
    }
}

fn package_json_children(base: PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    if !base.is_dir() {
        return Ok(Vec::new());
    }

    let mut files = std::fs::read_dir(base)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path().join("package.json"))
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn is_repository_analysis_prompt(prompt: &str) -> bool {
    let lowered = prompt.to_ascii_lowercase();
    contains_any(&lowered, ANALYSIS_KEYWORDS) && contains_any(&lowered, REPO_CONTEXT_KEYWORDS)
}

fn truncate_chars(content: &str, max_chars: usize) -> String {
    let mut result = content.chars().take(max_chars).collect::<String>();
    if content.chars().count() > max_chars {
        result.push_str("\n...[truncated]");
    }
    result
}

fn should_descend(entry: &DirEntry) -> bool {
    !ignored_name(&entry.file_name().to_string_lossy())
}

fn ignored_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".next"
            | ".openakta"
            | ".turbo"
            | "build"
            | "coverage"
            | "dist"
            | "node_modules"
            | "target"
            | ".DS_Store"
    )
}

fn contains_any(message: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|keyword| message.contains(keyword))
}

fn count_dependencies(message: &str) -> usize {
    ORDERING_KEYWORDS
        .iter()
        .filter(|keyword| message.contains(**keyword))
        .count()
}

fn estimate_deliverables(message: &str) -> usize {
    let delimiter_count = [",", " and ", " then ", " plus ", " also "]
        .iter()
        .filter(|needle| message.contains(**needle))
        .count();
    let explicit_deliverables = ["test", "tests", "docs", "documentation", "review"]
        .iter()
        .filter(|needle| message.contains(**needle))
        .count();
    1 + delimiter_count.max(explicit_deliverables)
}

fn is_trivial_direct_reply(
    message: &str,
    token_count: usize,
    side_effects_requested: bool,
) -> bool {
    if side_effects_requested {
        return false;
    }

    token_count <= 6
        || message.starts_with("say only ")
        || message.starts_with("reply only ")
        || message.starts_with("answer only ")
        || message == "hi"
        || message == "hello"
        || message == "say hi"
        || message == "just hi"
}

fn classify_risk(message: &str, broad_scope: bool, side_effects_requested: bool) -> RiskLevel {
    if contains_any(message, HIGH_RISK_KEYWORDS) || (broad_scope && side_effects_requested) {
        RiskLevel::High
    } else if side_effects_requested || broad_scope {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    }
}

const FILE_EXTENSIONS: &[&str] = &[".rs", ".ts", ".tsx", ".js", ".jsx", ".py", ".md", ".json"];
const SIDE_EFFECT_KEYWORDS: &[&str] = &[
    "fix ",
    "edit ",
    "update ",
    "change ",
    "create ",
    "delete ",
    "rename ",
    "refactor ",
    "write ",
    "run ",
    "execute ",
    "apply ",
    "patch ",
    "implement ",
];
const CODE_EDIT_KEYWORDS: &[&str] = &[
    "fix", "edit", "update", "change", "rename", "patch", "refactor",
];
const REPO_CONTEXT_KEYWORDS: &[&str] = &[
    "codebase",
    "repo",
    "repository",
    "workspace",
    "manifest",
    "package.json",
    "pnpm-workspace",
    "turbo",
    "apps/",
    "packages/",
    "file",
    "function",
    "module",
    "class",
    "why does",
    "where is",
    "how does",
    "in src/",
    "in crates/",
];
const ANALYSIS_KEYWORDS: &[&str] = &[
    "analyze",
    "analisa",
    "analyse",
    "explain",
    "explica",
    "review",
    "inspect",
    "read",
    "lê",
    "ler",
    "flow",
    "fluxo",
    "architecture",
    "arquitetura",
    "gargalos",
    "bottleneck",
    "inconsist",
];
const BROAD_SCOPE_KEYWORDS: &[&str] = &[
    "across",
    "codebase",
    "multiple",
    "all files",
    "entire",
    "end to end",
    "full flow",
];
const EXPLICIT_MULTI_PHASE_KEYWORDS: &[&str] = &[
    "step by step",
    "phase 1",
    "phase 2",
    "then ",
    "after that",
    "compare and contrast",
];
const ORDERING_KEYWORDS: &[&str] = &[" then ", " after ", " before ", " first ", " second "];
const HIGH_RISK_KEYWORDS: &[&str] = &[
    "delete",
    "drop",
    "migration",
    "database",
    "auth",
    "security",
    "deploy",
    "production",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn say_only_hi_routes_to_direct_reply() {
        let tmp = tempfile::tempdir().unwrap();
        let decision = MissionGate::analyze(&MissionGateRequest {
            message: "say only hi",
            workspace_root: tmp.path(),
            surface: MessageSurface::CliDo,
            response_preference: ResponsePreference::PreferMission,
            allow_code_context: true,
            side_effects_allowed: true,
            workspace_context_override: None,
        })
        .unwrap();

        assert_eq!(decision.mode, MessageExecutionMode::DirectReply);
        assert_eq!(decision.decomposition_budget.max_tasks, 1);
    }

    #[test]
    fn single_file_repo_question_stays_bounded() {
        let tmp = tempfile::tempdir().unwrap();
        let decision = MissionGate::analyze(&MissionGateRequest {
            message: "why does src/main.rs fail to parse?",
            workspace_root: tmp.path(),
            surface: MessageSurface::CliDo,
            response_preference: ResponsePreference::Auto,
            allow_code_context: true,
            side_effects_allowed: true,
            workspace_context_override: None,
        })
        .unwrap();

        assert!(matches!(
            decision.mode,
            MessageExecutionMode::DirectReply | MessageExecutionMode::SingleAgent
        ));
        assert_ne!(decision.mode, MessageExecutionMode::MultiStep);
    }

    #[test]
    fn one_file_edit_routes_to_direct_action() {
        let tmp = tempfile::tempdir().unwrap();
        let decision = MissionGate::analyze(&MissionGateRequest {
            message: "fix src/lib.rs to return hi",
            workspace_root: tmp.path(),
            surface: MessageSurface::CliDo,
            response_preference: ResponsePreference::PreferMission,
            allow_code_context: true,
            side_effects_allowed: true,
            workspace_context_override: None,
        })
        .unwrap();

        assert_eq!(decision.mode, MessageExecutionMode::DirectAction);
        assert_eq!(decision.task_type, TaskType::CodeModification);
    }

    #[test]
    fn codebase_wide_work_routes_to_multistep() {
        let tmp = tempfile::tempdir().unwrap();
        let decision = MissionGate::analyze(&MissionGateRequest {
            message: "refactor auth, database, and API layers across the codebase",
            workspace_root: tmp.path(),
            surface: MessageSurface::CliDo,
            response_preference: ResponsePreference::PreferMission,
            allow_code_context: true,
            side_effects_allowed: true,
            workspace_context_override: None,
        })
        .unwrap();

        assert_eq!(decision.mode, MessageExecutionMode::MultiStep);
        assert!(!decision.delegation_budget.allow_delegation);
    }

    #[test]
    fn read_only_repo_analysis_routes_to_single_agent() {
        let tmp = tempfile::tempdir().unwrap();
        let decision = MissionGate::analyze(&MissionGateRequest {
            message: "Lê este repositório e explica o fluxo real de entrada de mensagens até à resposta final, citando módulos principais, gargalos e inconsistências. Só análise; não escrever, editar ou aplicar patches.",
            workspace_root: tmp.path(),
            surface: MessageSurface::CliDo,
            response_preference: ResponsePreference::PreferMission,
            allow_code_context: true,
            side_effects_allowed: true,
            workspace_context_override: None,
        })
        .unwrap();

        assert_eq!(decision.mode, MessageExecutionMode::SingleAgent);
        assert_eq!(decision.retrieval_plan.max_hits, 4);
        assert_eq!(decision.task_type, TaskType::Retrieval);
    }

    #[test]
    fn extract_target_hints_recognizes_json_manifests() {
        let hints = extract_target_hints(
            "Read package.json, apps/auth/package.json, and packages/ui/package.json",
        );

        assert!(hints.target_files.contains(&"package.json".to_string()));
        assert!(hints
            .target_files
            .contains(&"apps/auth/package.json".to_string()));
        assert!(hints
            .target_files
            .contains(&"packages/ui/package.json".to_string()));
    }

    #[test]
    fn manifest_bootstrap_context_prefers_root_and_real_app_manifests() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("apps/auth/.next/dev")).unwrap();
        std::fs::create_dir_all(tmp.path().join("apps/bnf")).unwrap();
        std::fs::write(tmp.path().join("package.json"), "{ \"name\": \"root\" }").unwrap();
        std::fs::write(
            tmp.path().join("apps/auth/package.json"),
            "{ \"name\": \"auth\" }",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("apps/bnf/package.json"),
            "{ \"name\": \"bnf\" }",
        )
        .unwrap();
        std::fs::write(
            tmp.path().join("apps/auth/.next/dev/package.json"),
            "{ \"type\": \"commonjs\" }",
        )
        .unwrap();

        let context = build_workspace_context(
            tmp.path(),
            "Inspeciona o workspace em modo somente leitura. Lê package.json da raiz e os package.json dentro de apps/.",
            3,
        )
        .unwrap();

        assert!(context.contains("File: package.json"));
        assert!(context.contains("File: apps/auth/package.json"));
        assert!(context.contains("File: apps/bnf/package.json"));
        assert!(!context.contains(".next/dev/package.json"));
    }
}
