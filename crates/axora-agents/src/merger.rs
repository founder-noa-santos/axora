//! Result merging and conflict resolution for multi-worker outputs.

mod conflict_resolver;
mod result_combiner;

use crate::Result;
pub use conflict_resolver::{ConflictDetector, ConflictResolver};
pub use result_combiner::ResultCombiner;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Worker identifier used by merger inputs and outputs.
pub type WorkerId = String;

/// File content category used to choose merge behavior.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FileContentType {
    /// Source code or config-like text that should use three-way merging.
    Code,
    /// Documentation that can be concatenated and de-duplicated.
    Documentation,
    /// Opaque text content.
    Other,
}

/// Region of a file changed by a worker.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangeRegion {
    /// Inclusive start line, 1-based.
    pub start_line: usize,
    /// Inclusive end line, 1-based.
    pub end_line: usize,
}

impl ChangeRegion {
    /// Creates a new change region.
    pub fn new(start_line: usize, end_line: usize) -> Self {
        Self {
            start_line,
            end_line: end_line.max(start_line),
        }
    }

    /// Returns true when two regions overlap.
    pub fn overlaps(&self, other: &Self) -> bool {
        self.start_line <= other.end_line && other.start_line <= self.end_line
    }
}

/// Per-file worker output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileChange {
    /// Target file path.
    pub file_path: PathBuf,
    /// Full worker-produced content for the file.
    pub content: String,
    /// File content type.
    pub content_type: FileContentType,
    /// Approximate changed sections.
    pub changed_regions: Vec<ChangeRegion>,
}

impl FileChange {
    /// Creates a file change.
    pub fn new(
        file_path: impl Into<PathBuf>,
        content: impl Into<String>,
        content_type: FileContentType,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            content: content.into(),
            content_type,
            changed_regions: Vec::new(),
        }
    }

    /// Adds change regions.
    pub fn with_regions(mut self, regions: Vec<ChangeRegion>) -> Self {
        self.changed_regions = regions;
        self
    }
}

/// Merger-local worker task result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerTaskResult {
    /// Mission being merged.
    pub mission_id: String,
    /// Worker identifier.
    pub worker_id: WorkerId,
    /// Whether worker execution succeeded.
    pub success: bool,
    /// Plain output summary.
    pub output: String,
    /// File-level changes.
    pub file_changes: Vec<FileChange>,
    /// Dependency versions the worker assumed.
    pub dependency_versions: HashMap<String, String>,
    /// Named resources touched by the worker.
    pub resource_changes: Vec<String>,
    /// Confidence / quality score used for conservative conflict resolution.
    pub score: f32,
}

impl WorkerTaskResult {
    /// Creates a new worker result with sensible defaults.
    pub fn new(
        mission_id: impl Into<String>,
        worker_id: impl Into<String>,
        output: impl Into<String>,
    ) -> Self {
        Self {
            mission_id: mission_id.into(),
            worker_id: worker_id.into(),
            success: true,
            output: output.into(),
            file_changes: Vec::new(),
            dependency_versions: HashMap::new(),
            resource_changes: Vec::new(),
            score: 1.0,
        }
    }
}

/// Alias kept close to the sprint spec.
pub type TaskResult = WorkerTaskResult;

/// Base state used for three-way merges.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MergeState {
    /// File contents before worker execution.
    pub files: HashMap<PathBuf, String>,
    /// Dependency versions before worker execution.
    pub dependencies: HashMap<String, String>,
    /// Resource snapshots before worker execution.
    pub resources: HashMap<String, String>,
}

impl MergeState {
    /// Creates an empty base state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the base content for a file.
    pub fn file_content(&self, file_path: &Path) -> String {
        self.files.get(file_path).cloned().unwrap_or_default()
    }
}

/// Alias kept close to the sprint spec.
pub type State = MergeState;

/// Type of merge conflict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeConflictType {
    /// Both workers modified the same file.
    FileOverwrite,
    /// Both workers produced semantically incompatible changes.
    IncompatibleChanges,
    /// Workers used different dependency versions.
    DependencyMismatch,
    /// Both workers modified the same shared resource.
    ResourceConflict,
}

/// Conflict resolution state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MergeConflictResolution {
    /// Resolved automatically by merging.
    AutoMerged,
    /// Worker A version won.
    WorkerAWins,
    /// Worker B version won.
    WorkerBWins,
    /// Manual user intervention required.
    ManualResolution,
}

/// Merge conflict record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeConflict {
    /// Stable conflict identifier.
    pub conflict_id: String,
    /// Conflict category.
    pub conflict_type: MergeConflictType,
    /// File or synthetic path associated with the conflict.
    pub file_path: PathBuf,
    /// First worker.
    pub worker_a: WorkerId,
    /// Second worker.
    pub worker_b: WorkerId,
    /// Human-readable description.
    pub description: String,
    /// Resolution, if known.
    pub resolution: Option<MergeConflictResolution>,
}

/// Alias kept close to the sprint spec.
pub type Conflict = MergeConflict;
/// Alias kept close to the sprint spec.
pub type ConflictType = MergeConflictType;
/// Alias kept close to the sprint spec.
pub type ConflictResolution = MergeConflictResolution;

/// Merged file output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergedFile {
    /// File path.
    pub file_path: PathBuf,
    /// Base content before merging.
    pub base_version: String,
    /// Final merged content.
    pub merged_content: String,
    /// Whether unresolved conflicts remain.
    pub has_conflicts: bool,
    /// Workers who contributed to the merged file.
    pub contributors: Vec<WorkerId>,
}

/// Final merged result for a mission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedResult {
    /// Mission identifier.
    pub mission_id: String,
    /// Whether merge completed without escalated conflicts.
    pub success: bool,
    /// Combined text output.
    pub combined_output: String,
    /// All detected conflicts.
    pub conflicts: Vec<Conflict>,
    /// Number of conflicts resolved automatically.
    pub resolved_conflicts: usize,
    /// Number of conflicts escalated to the user.
    pub escalated_conflicts: usize,
    /// Merged file outputs.
    pub merged_files: Vec<MergedFile>,
}

/// Result merger configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergerConfig {
    /// Automatically resolve simple conflicts.
    pub auto_resolve_simple: bool,
    /// Escalate all conflicts when this threshold is exceeded.
    pub max_conflicts_before_escalate: usize,
    /// Use three-way merge for code-like content.
    pub use_three_way_merge: bool,
}

impl Default for MergerConfig {
    fn default() -> Self {
        Self {
            auto_resolve_simple: true,
            max_conflicts_before_escalate: 5,
            use_three_way_merge: true,
        }
    }
}

/// Unified result merger facade.
pub struct ResultMerger {
    combiner: ResultCombiner,
    detector: ConflictDetector,
    resolver: ConflictResolver,
    config: MergerConfig,
}

impl ResultMerger {
    /// Creates a new result merger.
    pub fn new(config: MergerConfig) -> Self {
        Self {
            combiner: ResultCombiner::new(config.use_three_way_merge),
            detector: ConflictDetector::new(),
            resolver: ConflictResolver::new(config.auto_resolve_simple),
            config,
        }
    }

    /// Merges worker results against a base state.
    pub fn merge(&self, results: Vec<TaskResult>, base_state: &State) -> Result<MergedResult> {
        let mut conflicts = self.detect_conflicts(&results);
        conflicts = self.auto_resolve_conflicts(conflicts, &results);

        if conflicts.len() > self.config.max_conflicts_before_escalate {
            conflicts = conflicts
                .into_iter()
                .map(|conflict| self.resolver.escalate_to_user(conflict))
                .collect();
        }

        self.combiner
            .combine_results(&results, base_state, conflicts)
    }

    /// Detects conflicts in worker results.
    pub fn detect_conflicts(&self, results: &[TaskResult]) -> Vec<Conflict> {
        self.detector.detect_conflicts(results)
    }

    /// Attempts auto-resolution for all conflicts.
    pub fn auto_resolve_conflicts(
        &self,
        conflicts: Vec<Conflict>,
        results: &[TaskResult],
    ) -> Vec<Conflict> {
        conflicts
            .into_iter()
            .map(|conflict| self.resolver.auto_resolve(conflict, results))
            .collect()
    }

    /// Returns conflicts that still require user intervention.
    pub fn get_escalated_conflicts(&self, merged_result: &MergedResult) -> Vec<Conflict> {
        merged_result
            .conflicts
            .iter()
            .filter(|conflict| conflict.resolution == Some(ConflictResolution::ManualResolution))
            .cloned()
            .collect()
    }
}

impl Default for ResultMerger {
    fn default() -> Self {
        Self::new(MergerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn worker_result(worker_id: &str, file_path: &str, content: &str) -> TaskResult {
        let mut result = TaskResult::new("mission-1", worker_id, format!("{worker_id} output"));
        result.file_changes.push(
            FileChange::new(file_path, content, FileContentType::Code)
                .with_regions(vec![ChangeRegion::new(2, 2)]),
        );
        result
    }

    #[test]
    fn test_merge_non_conflicting_files() {
        let merger = ResultMerger::default();
        let mut state = State::new();
        state
            .files
            .insert(PathBuf::from("a.rs"), "fn a() {}\n".to_string());
        state
            .files
            .insert(PathBuf::from("b.rs"), "fn b() {}\n".to_string());

        let results = vec![
            worker_result("worker-a", "a.rs", "fn a() { println!(\"a\"); }\n"),
            worker_result("worker-b", "b.rs", "fn b() { println!(\"b\"); }\n"),
        ];

        let merged = merger.merge(results, &state).unwrap();

        assert!(merged.success);
        assert_eq!(merged.merged_files.len(), 2);
        assert_eq!(merged.escalated_conflicts, 0);
    }

    #[test]
    fn test_detect_conflict_for_same_file() {
        let merger = ResultMerger::default();
        let results = vec![
            worker_result("worker-a", "shared.rs", "fn a() {}\n"),
            worker_result("worker-b", "shared.rs", "fn b() {}\n"),
        ];

        let conflicts = merger.detect_conflicts(&results);

        assert_eq!(conflicts.len(), 1);
        assert_eq!(
            conflicts[0].conflict_type,
            ConflictType::IncompatibleChanges
        );
    }

    #[test]
    fn test_auto_resolve_simple_doc_conflict() {
        let merger = ResultMerger::default();
        let mut a = TaskResult::new("mission-1", "worker-a", "docs a");
        a.file_changes.push(
            FileChange::new("README.md", "# Intro\nA\n", FileContentType::Documentation)
                .with_regions(vec![ChangeRegion::new(1, 2)]),
        );
        let mut b = TaskResult::new("mission-1", "worker-b", "docs b");
        b.file_changes.push(
            FileChange::new("README.md", "# Usage\nB\n", FileContentType::Documentation)
                .with_regions(vec![ChangeRegion::new(3, 4)]),
        );

        let conflicts = merger.auto_resolve_conflicts(merger.detect_conflicts(&[a, b]), &[]);
        assert_eq!(
            conflicts[0].resolution,
            Some(ConflictResolution::AutoMerged)
        );
    }

    #[test]
    fn test_escalate_complex_conflicts() {
        let merger = ResultMerger::new(MergerConfig {
            max_conflicts_before_escalate: 0,
            ..MergerConfig::default()
        });
        let mut state = State::new();
        state
            .files
            .insert(PathBuf::from("shared.rs"), "fn main() {}\n".to_string());
        let results = vec![
            worker_result("worker-a", "shared.rs", "fn main() { a(); }\n"),
            worker_result("worker-b", "shared.rs", "fn main() { b(); }\n"),
        ];

        let merged = merger.merge(results, &state).unwrap();
        assert_eq!(merged.escalated_conflicts, 1);
        assert!(!merger.get_escalated_conflicts(&merged).is_empty());
    }

    #[test]
    fn test_dependency_mismatch_conflict() {
        let merger = ResultMerger::default();
        let mut a = TaskResult::new("mission-1", "worker-a", "dep a");
        a.dependency_versions = HashMap::from([("serde".to_string(), "1.0.0".to_string())]);
        let mut b = TaskResult::new("mission-1", "worker-b", "dep b");
        b.dependency_versions = HashMap::from([("serde".to_string(), "1.0.1".to_string())]);

        let conflicts = merger.detect_conflicts(&[a, b]);
        assert_eq!(conflicts[0].conflict_type, ConflictType::DependencyMismatch);
    }
}
