//! Combines worker results into merged files and outputs.

use super::{
    ChangeRegion, Conflict, ConflictResolution, FileContentType, MergedFile, MergedResult, State,
    TaskResult,
};
use crate::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Combines worker outputs into final merged artifacts.
pub struct ResultCombiner {
    use_three_way_merge: bool,
}

impl ResultCombiner {
    /// Creates a new combiner.
    pub fn new(use_three_way_merge: bool) -> Self {
        Self {
            use_three_way_merge,
        }
    }

    /// Combines worker results into a merged result.
    pub fn combine_results(
        &self,
        results: &[TaskResult],
        base_state: &State,
        conflicts: Vec<Conflict>,
    ) -> Result<MergedResult> {
        let mission_id = results
            .first()
            .map(|result| result.mission_id.clone())
            .unwrap_or_else(|| "mission-unknown".to_string());

        let merged_files = self.combine_files(results, base_state, &conflicts);
        let combined_output = results
            .iter()
            .map(|result| format!("[{}]\n{}", result.worker_id, result.output))
            .collect::<Vec<_>>()
            .join("\n\n");

        let resolved_conflicts = conflicts
            .iter()
            .filter(|conflict| {
                matches!(
                    conflict.resolution,
                    Some(
                        ConflictResolution::AutoMerged
                            | ConflictResolution::WorkerAWins
                            | ConflictResolution::WorkerBWins
                    )
                )
            })
            .count();
        let escalated_conflicts = conflicts
            .iter()
            .filter(|conflict| conflict.resolution == Some(ConflictResolution::ManualResolution))
            .count();

        Ok(MergedResult {
            mission_id,
            success: escalated_conflicts == 0,
            combined_output,
            conflicts,
            resolved_conflicts,
            escalated_conflicts,
            merged_files,
        })
    }

    fn combine_files(
        &self,
        results: &[TaskResult],
        base_state: &State,
        conflicts: &[Conflict],
    ) -> Vec<MergedFile> {
        let mut by_file = BTreeMap::<PathBuf, Vec<&TaskResult>>::new();
        for result in results {
            for change in &result.file_changes {
                by_file
                    .entry(change.file_path.clone())
                    .or_default()
                    .push(result);
            }
        }

        by_file
            .into_iter()
            .map(|(file_path, contributors)| {
                let base_version = base_state.file_content(&file_path);
                let conflict_for_file = conflicts
                    .iter()
                    .filter(|conflict| conflict.file_path == file_path)
                    .collect::<Vec<_>>();
                let has_conflicts = conflict_for_file.iter().any(|conflict| {
                    conflict.resolution == Some(ConflictResolution::ManualResolution)
                        || conflict.resolution.is_none()
                });

                let merged_content = if contributors.len() == 1 {
                    contributor_change(contributors[0], &file_path)
                        .content
                        .clone()
                } else {
                    let changes = contributors
                        .iter()
                        .map(|result| contributor_change(result, &file_path))
                        .collect::<Vec<_>>();

                    match changes[0].content_type {
                        FileContentType::Documentation => self.merge_documentation(
                            &changes
                                .iter()
                                .map(|change| change.content.clone())
                                .collect::<Vec<_>>(),
                        ),
                        FileContentType::Code | FileContentType::Other
                            if self.use_three_way_merge =>
                        {
                            let mut merged = base_version.clone();
                            for change in &changes {
                                merged = self
                                    .merge_code_changes(&base_version, &merged, &change.content)
                                    .0;
                            }
                            merged
                        }
                        _ => changes
                            .last()
                            .map(|change| change.content.clone())
                            .unwrap_or_default(),
                    }
                };

                MergedFile {
                    file_path,
                    base_version,
                    merged_content,
                    has_conflicts,
                    contributors: contributors
                        .iter()
                        .map(|result| result.worker_id.clone())
                        .collect(),
                }
            })
            .collect()
    }

    /// Merges two worker variants against a base using a simple line-oriented three-way merge.
    pub fn merge_code_changes(&self, base: &str, worker_a: &str, worker_b: &str) -> (String, bool) {
        if worker_a == worker_b {
            return (worker_a.to_string(), false);
        }
        if worker_a == base {
            return (worker_b.to_string(), false);
        }
        if worker_b == base {
            return (worker_a.to_string(), false);
        }

        let Some(change_a) = diff_region(base, worker_a) else {
            return (worker_b.to_string(), false);
        };
        let Some(change_b) = diff_region(base, worker_b) else {
            return (worker_a.to_string(), false);
        };

        if !change_a.region.overlaps(&change_b.region) {
            let merged = apply_non_overlapping(base, &change_a, &change_b);
            return (merged, false);
        }

        let merged = format!(
            "<<<<<<< {}\n{}\n=======\n{}\n>>>>>>> {}\n",
            "worker-a", worker_a, worker_b, "worker-b"
        );
        (merged, true)
    }

    /// Merges documentation by collecting unique sections in stable order.
    pub fn merge_documentation(&self, docs: &[String]) -> String {
        let mut seen = BTreeSet::new();
        let mut sections = Vec::new();

        for doc in docs {
            for section in doc.split("\n\n") {
                let trimmed = section.trim();
                if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
                    sections.push(trimmed.to_string());
                }
            }
        }

        sections.join("\n\n")
    }
}

#[derive(Debug, Clone)]
struct DiffChange {
    region: ChangeRegion,
    replacement: Vec<String>,
}

fn contributor_change<'a>(result: &'a TaskResult, file_path: &PathBuf) -> &'a super::FileChange {
    result
        .file_changes
        .iter()
        .find(|change| &change.file_path == file_path)
        .expect("change must exist for contributor")
}

fn diff_region(base: &str, updated: &str) -> Option<DiffChange> {
    if base == updated {
        return None;
    }

    let base_lines = split_lines(base);
    let updated_lines = split_lines(updated);
    let prefix = common_prefix_len(&base_lines, &updated_lines);
    let base_suffix = common_suffix_len(&base_lines[prefix..], &updated_lines[prefix..]);
    let base_end = base_lines.len().saturating_sub(base_suffix);
    let updated_end = updated_lines.len().saturating_sub(base_suffix);

    Some(DiffChange {
        region: ChangeRegion::new(prefix + 1, base_end.max(prefix + 1)),
        replacement: updated_lines[prefix..updated_end].to_vec(),
    })
}

fn apply_non_overlapping(base: &str, first: &DiffChange, second: &DiffChange) -> String {
    let base_lines = split_lines(base);
    let (left, right) = if first.region.start_line <= second.region.start_line {
        (first, second)
    } else {
        (second, first)
    };

    let left_start = left.region.start_line.saturating_sub(1);
    let left_end = left.region.end_line.saturating_sub(1);
    let right_start = right.region.start_line.saturating_sub(1);
    let right_end = right.region.end_line.saturating_sub(1);

    let mut merged = Vec::new();
    merged.extend_from_slice(&base_lines[..left_start.min(base_lines.len())]);
    merged.extend(left.replacement.clone());
    if left_end + 1 <= right_start {
        merged.extend_from_slice(&base_lines[left_end + 1..right_start.min(base_lines.len())]);
    }
    merged.extend(right.replacement.clone());
    if right_end + 1 < base_lines.len() {
        merged.extend_from_slice(&base_lines[right_end + 1..]);
    }

    merged.join("\n")
}

fn split_lines(content: &str) -> Vec<String> {
    content.lines().map(str::to_string).collect()
}

fn common_prefix_len(a: &[String], b: &[String]) -> usize {
    a.iter()
        .zip(b.iter())
        .take_while(|(left, right)| left == right)
        .count()
}

fn common_suffix_len(a: &[String], b: &[String]) -> usize {
    a.iter()
        .rev()
        .zip(b.iter().rev())
        .take_while(|(left, right)| left == right)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merger::{ConflictType, FileChange, FileContentType, TaskResult};

    fn result(
        worker_id: &str,
        file: &str,
        content: &str,
        content_type: FileContentType,
    ) -> TaskResult {
        let mut result = TaskResult::new("mission-1", worker_id, format!("{worker_id} output"));
        result
            .file_changes
            .push(FileChange::new(file, content, content_type));
        result
    }

    #[test]
    fn test_merge_code_changes_prefers_non_modified_side() {
        let combiner = ResultCombiner::new(true);
        let (merged, has_conflict) = combiner.merge_code_changes("a\nb\n", "a\nb\n", "a\nc\n");

        assert_eq!(merged, "a\nc\n");
        assert!(!has_conflict);
    }

    #[test]
    fn test_merge_code_changes_merges_non_overlapping_regions() {
        let combiner = ResultCombiner::new(true);
        let base = "a\nb\nc\nd";
        let a = "a\nb1\nc\nd";
        let b = "a\nb\nc\nd1";
        let (merged, has_conflict) = combiner.merge_code_changes(base, a, b);

        assert!(merged.contains("b1"));
        assert!(merged.contains("d1"));
        assert!(!has_conflict);
    }

    #[test]
    fn test_merge_code_changes_marks_overlap_conflict() {
        let combiner = ResultCombiner::new(true);
        let (merged, has_conflict) = combiner.merge_code_changes("a\nb\n", "a\nx\n", "a\ny\n");

        assert!(merged.contains("<<<<<<<"));
        assert!(has_conflict);
    }

    #[test]
    fn test_merge_documentation_deduplicates_sections() {
        let combiner = ResultCombiner::new(true);
        let merged = combiner.merge_documentation(&[
            "# Intro\nA".to_string(),
            "# Intro\nA".to_string(),
            "# Usage\nB".to_string(),
        ]);

        assert_eq!(merged.matches("# Intro").count(), 1);
        assert!(merged.contains("# Usage"));
    }

    #[test]
    fn test_combine_results_builds_merged_files() {
        let combiner = ResultCombiner::new(true);
        let state = State::new();
        let conflicts = vec![Conflict {
            conflict_id: "c1".to_string(),
            conflict_type: ConflictType::FileOverwrite,
            file_path: PathBuf::from("a.rs"),
            worker_a: "worker-a".to_string(),
            worker_b: "worker-b".to_string(),
            description: "same file".to_string(),
            resolution: Some(ConflictResolution::ManualResolution),
        }];

        let merged = combiner
            .combine_results(
                &[
                    result("worker-a", "a.rs", "fn a() {}", FileContentType::Code),
                    result(
                        "worker-b",
                        "README.md",
                        "# Docs",
                        FileContentType::Documentation,
                    ),
                ],
                &state,
                conflicts,
            )
            .unwrap();

        assert_eq!(merged.merged_files.len(), 2);
        assert_eq!(merged.escalated_conflicts, 1);
    }
}
