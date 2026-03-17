//! Conflict detection and resolution for merger inputs.

use super::{Conflict, ConflictResolution, ConflictType, FileContentType, TaskResult};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Detects merge conflicts across worker results.
pub struct ConflictDetector;

impl ConflictDetector {
    /// Creates a new conflict detector.
    pub fn new() -> Self {
        Self
    }

    /// Detects conflicts across worker results.
    pub fn detect_conflicts(&self, results: &[TaskResult]) -> Vec<Conflict> {
        let mut conflicts = Vec::new();
        conflicts.extend(self.detect_file_conflicts(results));
        conflicts.extend(self.detect_dependency_conflicts(results));
        conflicts.extend(self.detect_resource_conflicts(results));
        conflicts
    }

    fn detect_file_conflicts(&self, results: &[TaskResult]) -> Vec<Conflict> {
        let mut by_file = BTreeMap::<PathBuf, Vec<(&TaskResult, &super::FileChange)>>::new();
        for result in results {
            for change in &result.file_changes {
                by_file
                    .entry(change.file_path.clone())
                    .or_default()
                    .push((result, change));
            }
        }

        let mut conflicts = Vec::new();
        for (file_path, changes) in by_file {
            if changes.len() < 2 {
                continue;
            }

            for idx in 0..changes.len() {
                for jdx in (idx + 1)..changes.len() {
                    let (left_result, left_change) = changes[idx];
                    let (right_result, right_change) = changes[jdx];

                    if left_change.content == right_change.content {
                        continue;
                    }

                    let overlapping_regions =
                        left_change.changed_regions.iter().any(|left_region| {
                            right_change
                                .changed_regions
                                .iter()
                                .any(|right_region| left_region.overlaps(right_region))
                        });

                    let conflict_type = if left_change.content_type
                        == FileContentType::Documentation
                        && right_change.content_type == FileContentType::Documentation
                        && !overlapping_regions
                    {
                        ConflictType::FileOverwrite
                    } else if overlapping_regions {
                        ConflictType::IncompatibleChanges
                    } else {
                        ConflictType::FileOverwrite
                    };

                    conflicts.push(Conflict {
                        conflict_id: format!(
                            "conflict-file-{}-{}-{}",
                            sanitize(&file_path),
                            left_result.worker_id,
                            right_result.worker_id
                        ),
                        conflict_type,
                        file_path: file_path.clone(),
                        worker_a: left_result.worker_id.clone(),
                        worker_b: right_result.worker_id.clone(),
                        description: format!(
                            "workers {} and {} both modified {}",
                            left_result.worker_id,
                            right_result.worker_id,
                            file_path.display()
                        ),
                        resolution: None,
                    });
                }
            }
        }

        conflicts
    }

    fn detect_dependency_conflicts(&self, results: &[TaskResult]) -> Vec<Conflict> {
        let mut by_dep = BTreeMap::<String, Vec<(&TaskResult, &String)>>::new();
        for result in results {
            for (name, version) in &result.dependency_versions {
                by_dep
                    .entry(name.clone())
                    .or_default()
                    .push((result, version));
            }
        }

        let mut conflicts = Vec::new();
        for (name, versions) in by_dep {
            if versions.len() < 2 {
                continue;
            }

            let first_version = versions[0].1;
            for (result, version) in versions.iter().skip(1) {
                if *version != first_version {
                    conflicts.push(Conflict {
                        conflict_id: format!("conflict-dep-{name}-{}", result.worker_id),
                        conflict_type: ConflictType::DependencyMismatch,
                        file_path: PathBuf::from("dependencies.lock"),
                        worker_a: versions[0].0.worker_id.clone(),
                        worker_b: result.worker_id.clone(),
                        description: format!(
                            "dependency {name} differs: {} vs {}",
                            first_version, version
                        ),
                        resolution: None,
                    });
                }
            }
        }
        conflicts
    }

    fn detect_resource_conflicts(&self, results: &[TaskResult]) -> Vec<Conflict> {
        let mut by_resource = BTreeMap::<String, Vec<&TaskResult>>::new();
        for result in results {
            for resource in &result.resource_changes {
                by_resource
                    .entry(resource.clone())
                    .or_default()
                    .push(result);
            }
        }

        let mut conflicts = Vec::new();
        for (resource, workers) in by_resource {
            if workers.len() < 2 {
                continue;
            }
            for idx in 0..workers.len() {
                for jdx in (idx + 1)..workers.len() {
                    conflicts.push(Conflict {
                        conflict_id: format!(
                            "conflict-resource-{}-{}-{}",
                            resource, workers[idx].worker_id, workers[jdx].worker_id
                        ),
                        conflict_type: ConflictType::ResourceConflict,
                        file_path: PathBuf::from(resource.clone()),
                        worker_a: workers[idx].worker_id.clone(),
                        worker_b: workers[jdx].worker_id.clone(),
                        description: format!(
                            "workers {} and {} both modified resource {}",
                            workers[idx].worker_id, workers[jdx].worker_id, resource
                        ),
                        resolution: None,
                    });
                }
            }
        }
        conflicts
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolves conflicts conservatively.
pub struct ConflictResolver {
    auto_resolve_simple: bool,
}

impl ConflictResolver {
    /// Creates a new conflict resolver.
    pub fn new(auto_resolve_simple: bool) -> Self {
        Self {
            auto_resolve_simple,
        }
    }

    /// Auto-resolves simple conflicts when safe.
    pub fn auto_resolve(&self, mut conflict: Conflict, results: &[TaskResult]) -> Conflict {
        if !self.auto_resolve_simple {
            return self.escalate_to_user(conflict);
        }

        conflict.resolution = match conflict.conflict_type {
            ConflictType::DependencyMismatch | ConflictType::ResourceConflict => {
                Some(ConflictResolution::ManualResolution)
            }
            ConflictType::FileOverwrite if is_documentation(&conflict.file_path) => {
                Some(ConflictResolution::AutoMerged)
            }
            ConflictType::FileOverwrite => best_worker_resolution(&conflict, results),
            ConflictType::IncompatibleChanges => {
                let resolution = best_worker_resolution(&conflict, results);
                if matches!(
                    resolution,
                    Some(ConflictResolution::WorkerAWins | ConflictResolution::WorkerBWins)
                ) {
                    resolution
                } else {
                    Some(ConflictResolution::ManualResolution)
                }
            }
        };

        if conflict.resolution.is_none() {
            conflict.resolution = Some(ConflictResolution::ManualResolution);
        }

        conflict
    }

    /// Marks a conflict for user escalation.
    pub fn escalate_to_user(&self, mut conflict: Conflict) -> Conflict {
        conflict.resolution = Some(ConflictResolution::ManualResolution);
        conflict
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(true)
    }
}

fn best_worker_resolution(
    conflict: &Conflict,
    results: &[TaskResult],
) -> Option<ConflictResolution> {
    let score_a = results
        .iter()
        .find(|result| result.worker_id == conflict.worker_a)
        .map(|result| result.score)
        .unwrap_or(0.0);
    let score_b = results
        .iter()
        .find(|result| result.worker_id == conflict.worker_b)
        .map(|result| result.score)
        .unwrap_or(0.0);

    let delta = (score_a - score_b).abs();
    if delta < 0.2 {
        return None;
    }

    if score_a > score_b {
        Some(ConflictResolution::WorkerAWins)
    } else {
        Some(ConflictResolution::WorkerBWins)
    }
}

fn is_documentation(path: &PathBuf) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext, "md" | "rst" | "txt"))
        .unwrap_or(false)
}

fn sanitize(path: &PathBuf) -> String {
    path.to_string_lossy().replace(['/', '.'], "-")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::merger::{ChangeRegion, FileChange, FileContentType, TaskResult};
    use std::collections::HashMap;

    fn worker(worker_id: &str) -> TaskResult {
        TaskResult::new("mission-1", worker_id, format!("{worker_id} output"))
    }

    #[test]
    fn test_detect_file_overwrite_conflict() {
        let detector = ConflictDetector::new();
        let mut a = worker("worker-a");
        a.file_changes.push(
            FileChange::new("shared.rs", "a", FileContentType::Code)
                .with_regions(vec![ChangeRegion::new(1, 1)]),
        );
        let mut b = worker("worker-b");
        b.file_changes.push(
            FileChange::new("shared.rs", "b", FileContentType::Code)
                .with_regions(vec![ChangeRegion::new(3, 3)]),
        );

        let conflicts = detector.detect_conflicts(&[a, b]);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::FileOverwrite);
    }

    #[test]
    fn test_detect_incompatible_conflict_for_overlap() {
        let detector = ConflictDetector::new();
        let mut a = worker("worker-a");
        a.file_changes.push(
            FileChange::new("shared.rs", "a", FileContentType::Code)
                .with_regions(vec![ChangeRegion::new(2, 4)]),
        );
        let mut b = worker("worker-b");
        b.file_changes.push(
            FileChange::new("shared.rs", "b", FileContentType::Code)
                .with_regions(vec![ChangeRegion::new(3, 5)]),
        );

        let conflicts = detector.detect_conflicts(&[a, b]);
        assert_eq!(
            conflicts[0].conflict_type,
            ConflictType::IncompatibleChanges
        );
    }

    #[test]
    fn test_detect_dependency_mismatch() {
        let detector = ConflictDetector::new();
        let mut a = worker("worker-a");
        a.dependency_versions = HashMap::from([("serde".to_string(), "1".to_string())]);
        let mut b = worker("worker-b");
        b.dependency_versions = HashMap::from([("serde".to_string(), "2".to_string())]);

        let conflicts = detector.detect_conflicts(&[a, b]);
        assert!(conflicts
            .iter()
            .any(|conflict| conflict.conflict_type == ConflictType::DependencyMismatch));
    }

    #[test]
    fn test_detect_resource_conflict() {
        let detector = ConflictDetector::new();
        let mut a = worker("worker-a");
        a.resource_changes = vec!["db:users".to_string()];
        let mut b = worker("worker-b");
        b.resource_changes = vec!["db:users".to_string()];

        let conflicts = detector.detect_conflicts(&[a, b]);
        assert!(conflicts
            .iter()
            .any(|conflict| conflict.conflict_type == ConflictType::ResourceConflict));
    }

    #[test]
    fn test_auto_resolve_docs_conflict() {
        let resolver = ConflictResolver::new(true);
        let conflict = Conflict {
            conflict_id: "c1".to_string(),
            conflict_type: ConflictType::FileOverwrite,
            file_path: PathBuf::from("README.md"),
            worker_a: "worker-a".to_string(),
            worker_b: "worker-b".to_string(),
            description: "doc conflict".to_string(),
            resolution: None,
        };

        let resolved = resolver.auto_resolve(conflict, &[]);
        assert_eq!(resolved.resolution, Some(ConflictResolution::AutoMerged));
    }

    #[test]
    fn test_escalate_complex_conflict() {
        let resolver = ConflictResolver::new(true);
        let conflict = Conflict {
            conflict_id: "c1".to_string(),
            conflict_type: ConflictType::ResourceConflict,
            file_path: PathBuf::from("db:users"),
            worker_a: "worker-a".to_string(),
            worker_b: "worker-b".to_string(),
            description: "resource conflict".to_string(),
            resolution: None,
        };

        let resolved = resolver.escalate_to_user(conflict);
        assert_eq!(
            resolved.resolution,
            Some(ConflictResolution::ManualResolution)
        );
    }
}
