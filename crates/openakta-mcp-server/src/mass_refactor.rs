//! Sandboxed mass-refactor orchestration.

use crate::execution::{
    CommandRequest, ContainerExecutor, ContainerExecutorConfig, ContainerMount, ExecutionOutcome,
    MassRefactorExecutorConfig,
};
use crate::McpError;
use openakta_cache::UnifiedDiff;
use openakta_indexing::MerkleTree;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

pub const MASS_REFACTOR_CONSENT_APPROVED: &str = "mass_script_approved";
pub const MASS_REFACTOR_SAFE_MODE_ID: &str = "safe_mode";
pub const MASS_REFACTOR_FAST_MODE_ID: &str = "mass_script_mode";
pub const MASS_REFACTOR_CONSENT_TEXT: &str = "Choose how to apply this codebase-wide refactor.\n\nOption A (Normal/Safe Mode): The LLM reads the files and generates deterministic unified diffs. Slower, consumes more tokens, but semantically safer.\n\nOption B (Mass Script Mode): The LLM generates a Python script and runs it in a sandboxed container against a staged workspace. Faster and more token-efficient, but a flawed script can overwrite intended staged logic across many files.\n\nApprove Mass Script Mode only if you want the refactor to run through the sandboxed Python workflow.";

#[derive(Debug, Clone)]
struct ApprovedRoot {
    relative_path: PathBuf,
    allow_descendants: bool,
}

/// Request for a staged mass refactor.
#[derive(Debug, Clone)]
pub struct MassRefactorRequest {
    pub session_id: String,
    pub workspace_root: PathBuf,
    pub target_paths: Vec<PathBuf>,
    pub script: String,
    pub timeout_secs: u32,
    pub consent_mode: String,
}

/// Final mass-refactor result.
#[derive(Debug, Clone)]
pub struct MassRefactorResult {
    pub success: bool,
    pub diff: String,
    pub changed_files: Vec<String>,
    pub stderr: String,
    pub rollback_performed: bool,
    pub execution: ExecutionOutcome,
}

/// Staged copy of the workspace scoped to approved targets.
#[derive(Debug, Clone)]
pub struct WorkspaceCheckpointer {
    workspace_root: PathBuf,
    session_root: PathBuf,
    baseline_root: PathBuf,
    stage_root: PathBuf,
    approved_roots: Vec<ApprovedRoot>,
}

impl WorkspaceCheckpointer {
    pub fn create(
        workspace_root: &Path,
        session_id: &str,
        target_paths: &[PathBuf],
    ) -> Result<Self, McpError> {
        let session_root = workspace_root
            .join(".openakta")
            .join("mass-refactor")
            .join(session_id);
        let baseline_root = session_root.join("baseline");
        let stage_root = session_root.join("workspace");
        fs::create_dir_all(&baseline_root).map_err(io_error)?;
        fs::create_dir_all(&stage_root).map_err(io_error)?;

        let mut approved_roots = Vec::new();
        for relative in target_paths {
            let live_path = workspace_root.join(relative);
            let allow_descendants = live_path.is_dir();
            approved_roots.push(ApprovedRoot {
                relative_path: relative.clone(),
                allow_descendants,
            });

            if live_path.is_dir() {
                for entry in WalkDir::new(&live_path).into_iter().filter_map(Result::ok) {
                    if !entry.file_type().is_file() {
                        continue;
                    }
                    let rel = entry
                        .path()
                        .strip_prefix(workspace_root)
                        .map_err(|err| McpError::ToolExecution(err.to_string()))?
                        .to_path_buf();
                    copy_live_file(workspace_root, &baseline_root, &rel)?;
                    copy_live_file(workspace_root, &stage_root, &rel)?;
                }
                continue;
            }

            if live_path.is_file() {
                copy_live_file(workspace_root, &baseline_root, relative)?;
                copy_live_file(workspace_root, &stage_root, relative)?;
            } else {
                ensure_parent_dir(&baseline_root.join(relative))?;
                ensure_parent_dir(&stage_root.join(relative))?;
            }
        }

        Ok(Self {
            workspace_root: workspace_root.to_path_buf(),
            session_root,
            baseline_root,
            stage_root,
            approved_roots,
        })
    }

    pub fn stage_root(&self) -> &Path {
        &self.stage_root
    }

    pub fn baseline_root(&self) -> &Path {
        &self.baseline_root
    }

    pub fn session_root(&self) -> &Path {
        &self.session_root
    }

    pub fn validate_stage_tree(&self) -> Result<(), McpError> {
        for entry in WalkDir::new(&self.stage_root)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let relative = entry
                .path()
                .strip_prefix(&self.stage_root)
                .map_err(|err| McpError::ToolExecution(err.to_string()))?;
            if !self.is_allowed(relative) {
                return Err(McpError::ToolExecution(format!(
                    "script wrote outside approved target_paths: {}",
                    relative.display()
                )));
            }
        }
        Ok(())
    }

    pub fn rollback(&self) -> Result<(), McpError> {
        if self.session_root.exists() {
            fs::remove_dir_all(&self.session_root).map_err(io_error)?;
        }
        Ok(())
    }

    pub fn commit(&self, changed_files: &[PathBuf]) -> Result<(), McpError> {
        let mut applied = Vec::new();
        for relative in changed_files {
            match self.promote_one(relative) {
                Ok(()) => applied.push(relative.clone()),
                Err(err) => {
                    self.restore_live(&applied)?;
                    return Err(err);
                }
            }
        }
        self.rollback()?;
        Ok(())
    }

    fn promote_one(&self, relative: &Path) -> Result<(), McpError> {
        let staged = self.stage_root.join(relative);
        let live = self.workspace_root.join(relative);
        if staged.exists() {
            ensure_parent_dir(&live)?;
            fs::copy(&staged, &live).map_err(io_error)?;
        } else if live.exists() {
            fs::remove_file(&live).map_err(io_error)?;
        }
        Ok(())
    }

    fn restore_live(&self, changed_files: &[PathBuf]) -> Result<(), McpError> {
        for relative in changed_files {
            let baseline = self.baseline_root.join(relative);
            let live = self.workspace_root.join(relative);
            if baseline.exists() {
                ensure_parent_dir(&live)?;
                fs::copy(&baseline, &live).map_err(io_error)?;
            } else if live.exists() {
                fs::remove_file(&live).map_err(io_error)?;
            }
        }
        Ok(())
    }

    fn is_allowed(&self, relative: &Path) -> bool {
        self.approved_roots.iter().any(|root| {
            relative == root.relative_path
                || (root.allow_descendants && relative.starts_with(&root.relative_path))
        })
    }
}

/// Workspace-wide diff synthesis for staged refactors.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceDiffGenerator;

impl WorkspaceDiffGenerator {
    pub fn generate(
        checkpointer: &WorkspaceCheckpointer,
    ) -> Result<(Vec<PathBuf>, String), McpError> {
        let old_tree = MerkleTree::build(checkpointer.baseline_root())
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let new_tree = MerkleTree::build(checkpointer.stage_root())
            .map_err(|err| McpError::ToolExecution(err.to_string()))?;
        let mut changed_files = new_tree.find_changed(&old_tree);
        changed_files.sort();

        let mut rendered = Vec::new();
        for relative in &changed_files {
            let old_content = read_utf8_or_empty(&checkpointer.baseline_root.join(relative))?;
            let new_content = read_utf8_or_empty(&checkpointer.stage_root.join(relative))?;
            let relative_str = relative.to_string_lossy().to_string();
            let diff =
                UnifiedDiff::generate(&old_content, &new_content, &relative_str, &relative_str);
            if !diff.hunks.is_empty() {
                rendered.push(diff.to_string());
            }
        }

        Ok((changed_files, rendered.join("\n")))
    }
}

/// Container-only facade for sandboxed Python refactors.
#[derive(Debug, Clone)]
pub struct MassRefactorTool {
    config: MassRefactorExecutorConfig,
}

impl MassRefactorTool {
    pub fn new(config: MassRefactorExecutorConfig) -> Self {
        Self { config }
    }

    pub async fn execute(
        &self,
        request: MassRefactorRequest,
    ) -> Result<MassRefactorResult, McpError> {
        if request.consent_mode != MASS_REFACTOR_CONSENT_APPROVED {
            return Err(McpError::ToolExecution(format!(
                "mass_refactor requires consent_mode={MASS_REFACTOR_CONSENT_APPROVED}"
            )));
        }

        let checkpointer = WorkspaceCheckpointer::create(
            &request.workspace_root,
            &request.session_id,
            &request.target_paths,
        )?;
        let script_path = checkpointer.session_root().join("mass_refactor.py");
        fs::write(&script_path, &request.script).map_err(io_error)?;

        let container = ContainerExecutor::new(ContainerExecutorConfig {
            runtime_binary: self.config.runtime_binary.clone(),
            image: self.config.image.clone(),
            workspace_mount_path: self.config.workspace_mount_path.clone(),
            extra_args: self.config.extra_args.clone(),
        });
        let execution = container
            .run_command_with_mounts(
                CommandRequest {
                    program: self.config.python_bin.clone(),
                    args: vec!["/tmp/openakta-mass-refactor.py".to_string()],
                    workspace_root: checkpointer.stage_root().to_path_buf(),
                    timeout_secs: request.timeout_secs,
                },
                &[ContainerMount {
                    host_path: script_path,
                    container_path: "/tmp/openakta-mass-refactor.py".to_string(),
                    read_only: true,
                }],
                Some(&self.config.workspace_mount_path),
            )
            .await?;

        if !execution.success {
            checkpointer.rollback()?;
            return Ok(MassRefactorResult {
                success: false,
                diff: String::new(),
                changed_files: normalize_target_paths(&request.target_paths),
                stderr: execution.stderr.clone(),
                rollback_performed: true,
                execution,
            });
        }

        if let Err(err) = checkpointer.validate_stage_tree() {
            checkpointer.rollback()?;
            return Ok(MassRefactorResult {
                success: false,
                diff: String::new(),
                changed_files: normalize_target_paths(&request.target_paths),
                stderr: err.to_string(),
                rollback_performed: true,
                execution,
            });
        }

        let (changed_files, diff) = match WorkspaceDiffGenerator::generate(&checkpointer) {
            Ok(result) => result,
            Err(err) => {
                checkpointer.rollback()?;
                return Ok(MassRefactorResult {
                    success: false,
                    diff: String::new(),
                    changed_files: normalize_target_paths(&request.target_paths),
                    stderr: err.to_string(),
                    rollback_performed: true,
                    execution,
                });
            }
        };

        if let Err(err) = checkpointer.commit(&changed_files) {
            checkpointer.rollback()?;
            return Ok(MassRefactorResult {
                success: false,
                diff: String::new(),
                changed_files: changed_files
                    .iter()
                    .map(|path| path.to_string_lossy().to_string())
                    .collect(),
                stderr: err.to_string(),
                rollback_performed: true,
                execution,
            });
        }

        Ok(MassRefactorResult {
            success: true,
            diff,
            changed_files: changed_files
                .iter()
                .map(|path| path.to_string_lossy().to_string())
                .collect(),
            stderr: execution.stderr.clone(),
            rollback_performed: false,
            execution,
        })
    }
}

fn copy_live_file(
    workspace_root: &Path,
    destination_root: &Path,
    relative: &Path,
) -> Result<(), McpError> {
    let source = workspace_root.join(relative);
    let destination = destination_root.join(relative);
    ensure_parent_dir(&destination)?;
    fs::copy(source, destination).map_err(io_error)?;
    Ok(())
}

fn ensure_parent_dir(path: &Path) -> Result<(), McpError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    Ok(())
}

fn read_utf8_or_empty(path: &Path) -> Result<String, McpError> {
    if !path.exists() {
        return Ok(String::new());
    }
    fs::read_to_string(path).map_err(|err| {
        McpError::ToolExecution(format!(
            "failed to read UTF-8 file {}: {err}",
            path.display()
        ))
    })
}

fn io_error(err: std::io::Error) -> McpError {
    McpError::ToolExecution(err.to_string())
}

fn normalize_target_paths(paths: &[PathBuf]) -> Vec<String> {
    let mut deduped = BTreeSet::new();
    for path in paths {
        deduped.insert(path.to_string_lossy().to_string());
    }
    deduped.into_iter().collect()
}

pub fn next_mass_refactor_session_id() -> String {
    format!("session-{}", Uuid::new_v4())
}

pub fn relative_targets_map(paths: &[PathBuf]) -> HashMap<String, PathBuf> {
    paths
        .iter()
        .map(|path| (path.to_string_lossy().to_string(), path.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn checkpointer_stages_only_approved_paths() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "fn demo() {}\n").unwrap();
        fs::write(temp_dir.path().join("README.md"), "ignored\n").unwrap();

        let checkpoint =
            WorkspaceCheckpointer::create(temp_dir.path(), "cp-1", &[PathBuf::from("src/lib.rs")])
                .unwrap();

        assert!(checkpoint.stage_root().join("src/lib.rs").exists());
        assert!(!checkpoint.stage_root().join("README.md").exists());
    }

    #[test]
    fn checkpointer_commit_promotes_and_rollback_cleans_session() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "fn before() {}\n").unwrap();

        let checkpoint =
            WorkspaceCheckpointer::create(temp_dir.path(), "cp-2", &[PathBuf::from("src/lib.rs")])
                .unwrap();
        fs::write(
            checkpoint.stage_root().join("src/lib.rs"),
            "fn after() {}\n",
        )
        .unwrap();

        checkpoint.commit(&[PathBuf::from("src/lib.rs")]).unwrap();

        assert_eq!(
            fs::read_to_string(temp_dir.path().join("src/lib.rs")).unwrap(),
            "fn after() {}\n"
        );
        assert!(!checkpoint.session_root().exists());
    }

    #[test]
    fn diff_generator_concatenates_changes() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "fn before() {}\n").unwrap();

        let checkpoint =
            WorkspaceCheckpointer::create(temp_dir.path(), "cp-3", &[PathBuf::from("src/lib.rs")])
                .unwrap();
        fs::write(
            checkpoint.stage_root().join("src/lib.rs"),
            "fn after() {}\n",
        )
        .unwrap();

        let (changed_files, diff) = WorkspaceDiffGenerator::generate(&checkpoint).unwrap();
        assert_eq!(changed_files, vec![PathBuf::from("src/lib.rs")]);
        assert!(diff.contains("-fn before() {}"));
        assert!(diff.contains("+fn after() {}"));
    }

    #[test]
    fn validate_stage_tree_rejects_unapproved_writes() {
        let temp_dir = TempDir::new().unwrap();
        fs::create_dir_all(temp_dir.path().join("src")).unwrap();
        fs::write(temp_dir.path().join("src/lib.rs"), "fn before() {}\n").unwrap();

        let checkpoint =
            WorkspaceCheckpointer::create(temp_dir.path(), "cp-4", &[PathBuf::from("src/lib.rs")])
                .unwrap();
        fs::create_dir_all(checkpoint.stage_root().join("src/extra")).unwrap();
        fs::write(
            checkpoint.stage_root().join("src/extra/new.rs"),
            "fn bad() {}\n",
        )
        .unwrap();

        assert!(checkpoint.validate_stage_tree().is_err());
    }
}
