use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use tracing::{debug, info, warn};

use openakta_docs::{
    append_changelog_entry, sanitize_segment, write_external_migration_file, CodeRealityIndex,
    ConfidenceReconcileDecision, ConfidenceScorer, DocExpectationIndex, DocReconciler,
    DocReconcilerConfig, DriftDetector, DriftDomain, DriftReport, IncrementalAstParser,
    MigrationChangeKind, ReconcileDecision, ToonChangelogPayload,
};
use openakta_indexing::MerkleTree;
use openakta_memory::{DocType, PersistentSemanticStore, SemanticMemory, SemanticMetadata};

use crate::background::governor::ResourceGovernor;
use crate::background::livingdocs_git;
use crate::background::queue::{
    resolved_status_for_choice, JobKind, JobRecord, PendingResolutionWorkItem, SqliteJobQueue,
    SyncScope,
};
use crate::background::review_resolution::{
    CodeResolutionFlag, CodeResolutionRequest, CodeResolutionRunner,
};

struct AutocommitOutcome {
    toon_changelog_entry_id: Option<String>,
    canonical_toon_json: Option<String>,
    external_relpath: Option<String>,
    commit_id: Option<String>,
    autocommit_error: Option<String>,
}

pub struct LivingDocsProcessor {
    workspace_root: PathBuf,
    docs_root: PathBuf,
    reconciler: DocReconciler,
    semantic_store: PersistentSemanticStore,
    job_queue: SqliteJobQueue,
    merkle_state_path: PathBuf,
    merkle_state: MerkleTree,
    doc_expectations: DocExpectationIndex,
    code_reality: CodeRealityIndex,
    ast_parser: IncrementalAstParser,
    drift_state_ready: bool,
    confidence_scorer: ConfidenceScorer,
    code_resolution_runner: Arc<dyn CodeResolutionRunner>,
}

impl LivingDocsProcessor {
    pub fn new(
        workspace_root: PathBuf,
        semantic_store_path: PathBuf,
        job_queue: SqliteJobQueue,
        ast_budget_bytes: usize,
        code_resolution_runner: Arc<dyn CodeResolutionRunner>,
    ) -> Result<Self> {
        let merkle_state_path = workspace_root
            .join(".openakta")
            .join("livingdocs-merkle.json");
        let merkle_state = MerkleTree::load_from_path(&merkle_state_path)
            .unwrap_or_else(|_| empty_tree(&workspace_root));
        let docs_root = workspace_root.join("akta-docs");
        let doc_expectations = if docs_root.exists() {
            DocExpectationIndex::parse_from_root(&docs_root)?
        } else {
            DocExpectationIndex::default()
        };

        Ok(Self {
            reconciler: DocReconciler::new(DocReconcilerConfig::new(workspace_root.clone())),
            workspace_root,
            docs_root,
            semantic_store: PersistentSemanticStore::new(&semantic_store_path, 384)
                .map_err(anyhow::Error::msg)?,
            job_queue,
            merkle_state_path,
            merkle_state,
            doc_expectations,
            code_reality: CodeRealityIndex::default(),
            ast_parser: IncrementalAstParser::new(ast_budget_bytes),
            drift_state_ready: false,
            confidence_scorer: ConfidenceScorer::with_defaults(),
            code_resolution_runner,
        })
    }

    pub fn process(&mut self, job: &JobRecord, governor: &mut ResourceGovernor) -> Result<()> {
        match (&job.kind, &job.payload.scope) {
            (JobKind::IncrementalSync, SyncScope::Paths(paths)) => {
                if paths.iter().any(|path| self.path_affects_drift(path)) {
                    self.ensure_drift_state(governor)?;
                }
                self.process_paths(paths, governor)?;
            }
            (JobKind::RescanWorkspace, _) | (_, SyncScope::Rescan) => {
                self.process_rescan(governor)?;
            }
        }

        self.merkle_state.save_to_path(&self.merkle_state_path)?;
        Ok(())
    }

    pub fn release_idle_parser_state(&mut self) {
        self.ast_parser.release_all_trees();
    }

    pub fn process_review_resolution(&mut self, work: &PendingResolutionWorkItem) -> Result<()> {
        let report = self
            .job_queue
            .drift_report_by_report_id(&work.report_id)?
            .ok_or_else(|| anyhow!("missing drift report {}", work.report_id))?;

        match work.choice.as_str() {
            "update_doc" => self.apply_doc_resolution(work, &report),
            "update_code" => self.apply_code_resolution(work, &report),
            other => anyhow::bail!("unknown resolution choice: {other}"),
        }
    }

    fn process_paths(&mut self, paths: &[String], governor: &mut ResourceGovernor) -> Result<()> {
        let mut drift_dirty = false;
        for relative in paths {
            governor.wait_for_budget();

            let absolute = self.workspace_root.join(relative);
            if !absolute.exists() {
                self.reconcile_deleted_file(Path::new(relative))?;
                drift_dirty |= self.remove_drift_path(relative, &absolute)?;
                governor.cooperative_yield();
                continue;
            }
            if !is_relevant_source(&absolute) {
                continue;
            }

            self.reconcile_file(relative, &absolute)?;
            drift_dirty |= self.update_drift_for_path(relative, &absolute)?;
            self.enforce_parser_pressure(governor);
            governor.cooperative_yield();
        }
        if drift_dirty {
            self.emit_drift_report("incremental_sync", governor)?;
        }
        Ok(())
    }

    fn process_rescan(&mut self, governor: &mut ResourceGovernor) -> Result<()> {
        let workspace_root = self.workspace_root.clone();
        self.walk_and_reconcile(&workspace_root, governor)?;

        let tracked_paths: Vec<PathBuf> = self.merkle_state.file_hashes.keys().cloned().collect();
        for relative in tracked_paths {
            if self.workspace_root.join(&relative).exists() {
                continue;
            }

            governor.wait_for_budget();
            self.reconcile_deleted_file(&relative)?;
            governor.cooperative_yield();
        }
        self.refresh_doc_expectations()?;
        self.rebuild_drift_state(governor)?;
        self.emit_drift_report("rescan_workspace", governor)?;
        Ok(())
    }

    fn reconcile_file(&mut self, relative: &str, absolute: &Path) -> Result<()> {
        let Ok(new_content) = fs::read_to_string(absolute) else {
            return Ok(());
        };

        let previous_signature = self.signature_snapshot(Path::new(relative));
        self.merkle_state.update(absolute, new_content.as_bytes())?;
        let current_signature = self.signature_snapshot(Path::new(relative));
        if previous_signature.hash == current_signature.hash {
            return Ok(());
        }

        let (decision, patches) = self.reconciler.reconcile_change(
            Path::new(relative),
            &previous_signature.body,
            &current_signature.body,
        );
        if decision == ReconcileDecision::Noop {
            return Ok(());
        }

        self.persist_semantic_memory(relative, &decision, &current_signature.body, patches.len())?;
        info!(
            file = relative,
            patch_count = patches.len(),
            decision = ?decision,
            "livingdocs generated reconciliation candidates"
        );
        for patch in patches {
            warn!(
                file = relative,
                target = %patch.target.display(),
                "livingdocs patch candidate ready"
            );
        }

        Ok(())
    }

    fn reconcile_deleted_file(&mut self, relative: &Path) -> Result<()> {
        let previous_signature = self.signature_snapshot(relative);
        if previous_signature.body.is_empty() {
            return Ok(());
        }

        self.remove_from_merkle(relative);
        let (decision, patches) =
            self.reconciler
                .reconcile_change(relative, &previous_signature.body, "");
        if decision == ReconcileDecision::Noop {
            return Ok(());
        }

        self.persist_semantic_memory(
            &relative.to_string_lossy(),
            &decision,
            "file removed",
            patches.len(),
        )?;
        for patch in patches {
            warn!(
                file = %relative.display(),
                target = %patch.target.display(),
                "livingdocs patch candidate ready for deleted file"
            );
        }
        Ok(())
    }

    fn persist_semantic_memory(
        &self,
        relative: &str,
        decision: &ReconcileDecision,
        signature: &str,
        patch_count: usize,
    ) -> Result<()> {
        let doc_type = if relative.contains("README") {
            DocType::UserGuide
        } else {
            DocType::ArchitecturalDoc
        };
        let content = format!(
            "LivingDocs sync\nsource: {relative}\ndecision: {}\npatch_count: {patch_count}\n\n{signature}",
            decision_tag(decision)
        );
        let memory_id = format!("livingdocs:{}", blake3::hash(relative.as_bytes()).to_hex());
        let now = now_secs();
        let metadata = SemanticMetadata::with_timestamps("living_docs", doc_type, now, now)
            .with_tag("doc_sync")
            .with_tag(decision_tag(decision))
            .with_related(relative);
        let memory = SemanticMemory::new(&memory_id, &content, embed_text(&content, 384), metadata);
        self.semantic_store
            .insert(memory)
            .map_err(anyhow::Error::msg)
    }

    fn signature_snapshot(&self, relative: &Path) -> SignatureSnapshot {
        let mut blocks: Vec<_> = self
            .merkle_state
            .block_hashes
            .values()
            .filter(|entry| entry.file_path == relative)
            .collect();
        blocks.sort_by(|left, right| {
            left.symbol_path
                .cmp(&right.symbol_path)
                .then(left.line_range.0.cmp(&right.line_range.0))
                .then(left.hash.cmp(&right.hash))
        });

        let file_hash = self
            .merkle_state
            .file_hashes
            .get(relative)
            .map(|entry| entry.hash.clone())
            .unwrap_or_default();
        let mut body = if file_hash.is_empty() {
            String::new()
        } else {
            format!("file: {}\nfile_hash: {file_hash}\n", relative.display())
        };

        for block in blocks {
            let symbol = block
                .symbol_path
                .clone()
                .unwrap_or_else(|| "<anonymous>".to_string());
            body.push_str(&format!(
                "symbol: {symbol}\nlanguage: {}\nlines: {}-{}\nblock_hash: {}\n",
                block.language, block.line_range.0, block.line_range.1, block.hash
            ));
        }

        SignatureSnapshot {
            hash: blake3::hash(body.as_bytes()).to_hex().to_string(),
            body,
        }
    }

    fn remove_from_merkle(&mut self, relative: &Path) {
        self.merkle_state.file_hashes.remove(relative);
        self.merkle_state
            .block_hashes
            .retain(|_, entry| entry.file_path != relative);
    }

    fn ensure_drift_state(&mut self, governor: &mut ResourceGovernor) -> Result<()> {
        if self.drift_state_ready || !self.docs_root.exists() {
            return Ok(());
        }

        self.refresh_doc_expectations()?;
        self.rebuild_drift_state(governor)?;
        Ok(())
    }

    fn refresh_doc_expectations(&mut self) -> Result<()> {
        self.doc_expectations = if self.docs_root.exists() {
            DocExpectationIndex::parse_from_root(&self.docs_root)?
        } else {
            DocExpectationIndex::default()
        };
        Ok(())
    }

    fn rebuild_drift_state(&mut self, governor: &mut ResourceGovernor) -> Result<()> {
        self.code_reality = CodeRealityIndex::default();
        self.ast_parser.release_all_trees();
        let workspace_root = self.workspace_root.clone();
        self.walk_and_index_drift(&workspace_root, governor)?;
        self.drift_state_ready = true;
        Ok(())
    }

    fn walk_and_index_drift(&mut self, root: &Path, governor: &mut ResourceGovernor) -> Result<()> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if is_ignored_dir_name(
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default(),
                ) {
                    continue;
                }
                self.walk_and_index_drift(&path, governor)?;
                continue;
            }
            if !is_drift_code_source(&path) {
                continue;
            }
            governor.wait_for_budget();
            self.update_code_snapshot(&path)?;
            self.enforce_parser_pressure(governor);
            governor.cooperative_yield();
        }
        Ok(())
    }

    fn update_drift_for_path(&mut self, relative: &str, absolute: &Path) -> Result<bool> {
        if is_drift_doc_relative(relative) {
            self.refresh_doc_expectations()?;
            return Ok(true);
        }
        if is_drift_code_source(absolute) {
            self.update_code_snapshot(absolute)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn remove_drift_path(&mut self, relative: &str, absolute: &Path) -> Result<bool> {
        if is_drift_doc_relative(relative) {
            self.refresh_doc_expectations()?;
            return Ok(true);
        }
        if is_drift_code_relative(relative) {
            self.ast_parser.invalidate(absolute);
            self.code_reality.remove_file(absolute);
            return Ok(true);
        }
        Ok(false)
    }

    fn update_code_snapshot(&mut self, absolute: &Path) -> Result<()> {
        let Ok(source) = fs::read_to_string(absolute) else {
            return Ok(());
        };
        let snapshot = self.ast_parser.parse_changed_file(absolute, &source)?;
        self.code_reality.upsert_snapshot(snapshot);
        Ok(())
    }

    fn emit_drift_report(&mut self, trigger: &str, governor: &mut ResourceGovernor) -> Result<()> {
        if self.doc_expectations.symbol_expectations.is_empty()
            && self.doc_expectations.code_references.is_empty()
        {
            return Ok(());
        }

        governor.wait_for_budget();
        let report = DriftDetector::detect(
            &self.doc_expectations,
            &self.code_reality,
            &self.workspace_root,
        );
        self.persist_drift_report(trigger, &report)?;
        if report.total_flags > 0 {
            warn!(
                trigger,
                total = report.total_flags,
                api_surface = report.api_surface_flags,
                business_rules = report.business_rule_flags,
                code_refs = report.code_reference_flags,
                highest_severity = ?report.highest_severity,
                "livingdocs drift detection flagged inconsistencies"
            );
        } else {
            debug!(
                trigger,
                "livingdocs drift detection found no inconsistencies"
            );
        }
        self.enforce_parser_pressure(governor);
        Ok(())
    }

    /// Drift confidence uses the same accept floor as the Evaluator Agent; only `UpdateRequired`
    /// reaches `try_autocommit_changelog` (no blind auto-commit).
    fn apply_confidence_routing(&self, report_id: &str, report: &DriftReport) -> Result<()> {
        let primary = self
            .confidence_scorer
            .primary_doc_path(report, &self.docs_root);
        let (decision, breakdown) = self.confidence_scorer.decide(report, primary);
        let breakdown_json = serde_json::to_string(&breakdown)?;
        let score = match &decision {
            ConfidenceReconcileDecision::Noop { .. } => 1.0,
            ConfidenceReconcileDecision::UpdateRequired { score, .. } => *score,
            ConfidenceReconcileDecision::ReviewRequired { score, .. } => *score,
        };
        let decision_tag = match &decision {
            ConfidenceReconcileDecision::Noop { .. } => "noop",
            ConfidenceReconcileDecision::UpdateRequired { .. } => "update_required",
            ConfidenceReconcileDecision::ReviewRequired { .. } => "review_required",
        };

        let mut toon: Option<String> = None;
        let mut commit: Option<String> = None;
        let mut external: Option<String> = None;
        let mut auto_err: Option<String> = None;

        match &decision {
            ConfidenceReconcileDecision::ReviewRequired { .. } => {
                if let Some(review_id) = self.job_queue.enqueue_reconcile_review(
                    report_id,
                    &self.workspace_root,
                    None,
                    score,
                    &breakdown_json,
                    "review_required",
                )? {
                    info!(
                        review_id = %review_id,
                        report_id = %report_id,
                        score,
                        "livingdocs queued reconcile review"
                    );
                }
            }
            ConfidenceReconcileDecision::UpdateRequired { target_docs, .. } => {
                let outcome =
                    self.try_autocommit_changelog(report_id, report, target_docs.as_slice())?;
                toon = outcome.canonical_toon_json;
                commit = outcome.commit_id;
                external = outcome.external_relpath;
                auto_err = outcome.autocommit_error;
            }
            ConfidenceReconcileDecision::Noop { .. } => {}
        }

        self.job_queue.record_confidence_action(
            report_id,
            &self.workspace_root,
            decision_tag,
            Some(score),
            &breakdown_json,
            toon.as_deref(),
            commit.as_deref(),
            external.as_deref(),
            auto_err.as_deref(),
        )?;
        Ok(())
    }

    fn try_autocommit_changelog(
        &self,
        report_id: &str,
        report: &DriftReport,
        targets: &[PathBuf],
    ) -> Result<AutocommitOutcome> {
        let Some(doc_path) = targets.first() else {
            return Ok(AutocommitOutcome {
                toon_changelog_entry_id: None,
                canonical_toon_json: None,
                external_relpath: None,
                commit_id: None,
                autocommit_error: Some("no_target_docs".into()),
            });
        };
        let rel = match doc_path.strip_prefix(&self.workspace_root) {
            Ok(r) => r,
            Err(_) => {
                return Ok(AutocommitOutcome {
                    toon_changelog_entry_id: None,
                    canonical_toon_json: None,
                    external_relpath: None,
                    commit_id: None,
                    autocommit_error: Some("doc_path_not_under_workspace".into()),
                });
            }
        };
        let rel_unix = rel.to_string_lossy().replace('\\', "/");
        let base =
            livingdocs_git::read_head_path(&self.workspace_root, &rel_unix).unwrap_or_default();
        let ts = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
        let short_id: String = report_id.chars().take(8).collect();
        let summary = format!("drift r{} flags={}", short_id, report.total_flags);
        let doc_seg = sanitize_segment(&rel_unix, 64);
        let slug_seg = sanitize_segment(&format!("drift-{}", short_id), 64);
        let payload = ToonChangelogPayload::from_parts(
            1,
            ts,
            MigrationChangeKind::Changed,
            summary,
            Some(doc_seg),
            Some(slug_seg),
        )?;
        let canonical_toon = serde_json::to_string(&payload)?;
        let toon_changelog_entry_id = Some(format!("{}-{}", payload.ts, payload.sha256_16));

        let mut external_relpath = None;
        if self.docs_root.exists() {
            match write_external_migration_file(&self.docs_root, &payload) {
                Ok(abs) => {
                    if let Ok(stripped) = abs.strip_prefix(&self.workspace_root) {
                        external_relpath = Some(stripped.to_string_lossy().replace('\\', "/"));
                    }
                }
                Err(e) => {
                    warn!(err = %e, "livingdocs external 10-changelog write failed");
                }
            }
        }

        let new_md = append_changelog_entry(&base, &payload, None)?;
        match livingdocs_git::try_commit_text_at_head(
            &self.workspace_root,
            &rel_unix,
            &new_md,
            &format!("livingdocs: drift changelog ({report_id})"),
        ) {
            Ok(Some(oid)) => {
                self.job_queue
                    .record_autocommit(&oid, report_id, &self.workspace_root)?;
                info!(commit = %oid, report_id = %report_id, "livingdocs alternate-index commit");
                Ok(AutocommitOutcome {
                    toon_changelog_entry_id,
                    canonical_toon_json: Some(canonical_toon),
                    external_relpath,
                    commit_id: Some(oid),
                    autocommit_error: None,
                })
            }
            Ok(None) => {
                warn!(
                    report_id = %report_id,
                    "livingdocs autocommit skipped (no git repo, empty repo, or no HEAD)"
                );
                Ok(AutocommitOutcome {
                    toon_changelog_entry_id,
                    canonical_toon_json: Some(canonical_toon),
                    external_relpath,
                    commit_id: None,
                    autocommit_error: Some("autocommit_skipped_no_git_or_head".into()),
                })
            }
            Err(e) => {
                warn!(?e, report_id = %report_id, "livingdocs git commit failed");
                Ok(AutocommitOutcome {
                    toon_changelog_entry_id,
                    canonical_toon_json: Some(canonical_toon),
                    external_relpath,
                    commit_id: None,
                    autocommit_error: Some(format!("git_error:{e}")),
                })
            }
        }
    }

    fn apply_doc_resolution(
        &self,
        work: &PendingResolutionWorkItem,
        report: &crate::background::queue::StoredDriftReport,
    ) -> Result<()> {
        let target_docs = report
            .flags
            .first()
            .map(|flag| flag.doc_path.clone())
            .into_iter()
            .collect::<Vec<_>>();
        let runtime_report = stored_report_to_runtime_report(report);
        let outcome =
            self.try_autocommit_changelog(&work.report_id, &runtime_report, &target_docs)?;
        self.job_queue.record_confidence_action(
            &work.report_id,
            &work.workspace_root,
            "resolved_with_doc_update",
            Some(work.confidence_score),
            &work.breakdown_json,
            outcome.canonical_toon_json.as_deref(),
            outcome.commit_id.as_deref(),
            outcome.external_relpath.as_deref(),
            outcome.autocommit_error.as_deref(),
        )?;
        self.job_queue.complete_resolution(
            &work.review_id,
            resolved_status_for_choice(&work.choice)?,
            None,
            outcome.toon_changelog_entry_id.as_deref(),
            outcome.autocommit_error.as_deref(),
        )?;
        info!(
            review_id = %work.review_id,
            report_id = %work.report_id,
            choice = %work.choice,
            outcome = "resolved_with_doc_update",
            toon_changelog_entry_id = ?outcome.toon_changelog_entry_id,
            "livingdocs review resolution completed"
        );
        Ok(())
    }

    fn apply_code_resolution(
        &self,
        work: &PendingResolutionWorkItem,
        report: &crate::background::queue::StoredDriftReport,
    ) -> Result<()> {
        let code_paths = report
            .flags
            .iter()
            .filter_map(|flag| flag.code_path.as_ref())
            .filter_map(|path| path.strip_prefix(&work.workspace_root).ok())
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let request = CodeResolutionRequest {
            review_id: work.review_id.clone(),
            report_id: work.report_id.clone(),
            workspace_root: work.workspace_root.clone(),
            primary_doc_path: report
                .flags
                .first()
                .and_then(|flag| flag.doc_path.strip_prefix(&work.workspace_root).ok())
                .map(|path| path.to_string_lossy().replace('\\', "/")),
            code_paths,
            flags: report
                .flags
                .iter()
                .map(|flag| CodeResolutionFlag {
                    doc_path: flag
                        .doc_path
                        .strip_prefix(&work.workspace_root)
                        .unwrap_or(flag.doc_path.as_path())
                        .to_string_lossy()
                        .replace('\\', "/"),
                    code_path: flag.code_path.as_ref().map(|path| {
                        path.strip_prefix(&work.workspace_root)
                            .unwrap_or(path.as_path())
                            .to_string_lossy()
                            .replace('\\', "/")
                    }),
                    symbol_name: flag.symbol_name.clone(),
                    rule_ids: flag.rule_ids.clone(),
                    severity: drift_severity_tag(&flag.severity).to_string(),
                    kind: drift_kind_tag(&flag.kind).to_string(),
                    message: flag.message.clone(),
                })
                .collect(),
        };
        let result = self.code_resolution_runner.run(&request)?;
        self.job_queue.complete_resolution(
            &work.review_id,
            resolved_status_for_choice(&work.choice)?,
            Some(&result.patch_receipt_id),
            None,
            None,
        )?;
        info!(
            review_id = %work.review_id,
            report_id = %work.report_id,
            choice = %work.choice,
            outcome = "resolved_with_code_update",
            patch_receipt_id = %result.patch_receipt_id,
            "livingdocs review resolution completed"
        );
        Ok(())
    }

    fn persist_drift_report(&self, trigger: &str, report: &DriftReport) -> Result<()> {
        let (report_id, inserted) =
            self.job_queue
                .persist_drift_report(&self.workspace_root, trigger, report)?;
        if inserted {
            self.apply_confidence_routing(&report_id, report)?;
        }
        let now = now_secs();
        let doc_type = if report.business_rule_flags > 0 {
            DocType::BusinessRule
        } else {
            DocType::ArchitecturalDoc
        };
        let memory_id = drift_memory_id(&self.workspace_root);
        let content = format!(
            "LivingDocs drift detection\nworkspace: {}\ntrigger: {trigger}\nreport_id: {report_id}\nstorage: sqlite_primary_semantic_summary\nflags_total: {}\napi_surface_flags: {}\nbusiness_rule_flags: {}\ncode_reference_flags: {}\ncritical_flags: {}\nwarning_flags: {}\ninfo_flags: {}\nhighest_severity: {:?}\n\n{}",
            self.workspace_root.display(),
            report.total_flags,
            report.api_surface_flags,
            report.business_rule_flags,
            report.code_reference_flags,
            report.critical_flags,
            report.warning_flags,
            report.info_flags,
            report.highest_severity,
            render_drift_flags(report),
        );
        let metadata = SemanticMetadata::with_timestamps("living_docs", doc_type, now, now)
            .with_tag("doc_sync")
            .with_tag("drift_report")
            .with_tag(trigger)
            .with_tag(highest_severity_tag(report))
            .with_tag(if report.business_rule_flags > 0 {
                "business_rule_drift"
            } else {
                "api_surface_drift"
            });
        let memory = SemanticMemory::new(&memory_id, &content, embed_text(&content, 384), metadata);
        self.semantic_store
            .insert(memory)
            .map_err(anyhow::Error::msg)
    }

    fn enforce_parser_pressure(&mut self, governor: &mut ResourceGovernor) {
        if governor
            .current_rss_bytes()
            .is_some_and(|rss| rss >= governor.idle_rss_limit_bytes())
        {
            self.ast_parser.release_all_trees();
            return;
        }

        let stats = self.ast_parser.memory_stats();
        let max_ast_bytes = (governor.hard_rss_limit_bytes() / 5) as usize;
        if stats.budget_bytes > max_ast_bytes {
            self.ast_parser.set_memory_budget(max_ast_bytes);
        }
    }

    fn path_affects_drift(&self, relative: &str) -> bool {
        is_drift_doc_relative(relative) || is_drift_code_relative(relative)
    }

    fn walk_and_reconcile(&mut self, root: &Path, governor: &mut ResourceGovernor) -> Result<()> {
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if is_ignored_dir_name(
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or_default(),
                ) {
                    continue;
                }
                self.walk_and_reconcile(&path, governor)?;
                continue;
            }

            if !is_relevant_source(&path) {
                continue;
            }

            governor.wait_for_budget();
            let Ok(relative) = path.strip_prefix(&self.workspace_root) else {
                continue;
            };
            self.reconcile_file(&relative.to_string_lossy(), &path)?;
            governor.cooperative_yield();
        }

        Ok(())
    }
}

fn stored_report_to_runtime_report(
    report: &crate::background::queue::StoredDriftReport,
) -> DriftReport {
    DriftReport {
        total_flags: report.total_flags,
        api_surface_flags: report.api_surface_flags,
        business_rule_flags: report.business_rule_flags,
        code_reference_flags: report.code_reference_flags,
        critical_flags: report.critical_flags,
        warning_flags: report.warning_flags,
        info_flags: report.info_flags,
        highest_severity: report.highest_severity.clone(),
        flags: report
            .flags
            .iter()
            .map(|flag| openakta_docs::InconsistencyFlag {
                domain: flag.domain.clone(),
                kind: flag.kind.clone(),
                severity: flag.severity.clone(),
                message: flag.message.clone(),
                doc_path: flag.doc_path.clone(),
                code_path: flag.code_path.clone(),
                symbol_name: flag.symbol_name.clone(),
                rule_ids: flag.rule_ids.clone(),
                fingerprint: flag.fingerprint.clone(),
            })
            .collect(),
    }
}

fn drift_kind_tag(kind: &openakta_docs::DriftKind) -> &'static str {
    match kind {
        openakta_docs::DriftKind::MissingSymbol => "missing_symbol",
        openakta_docs::DriftKind::SignatureMismatch => "signature_mismatch",
        openakta_docs::DriftKind::MissingRuleBinding => "missing_rule_binding",
        openakta_docs::DriftKind::StructuralDrift => "structural_drift",
        openakta_docs::DriftKind::DeadCodeReference => "dead_code_reference",
    }
}

fn drift_severity_tag(severity: &openakta_docs::DriftSeverity) -> &'static str {
    match severity {
        openakta_docs::DriftSeverity::Critical => "critical",
        openakta_docs::DriftSeverity::Warning => "warning",
        openakta_docs::DriftSeverity::Info => "info",
    }
}

fn is_relevant_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "md" | "toml" | "json" | "yaml" | "yml")
    )
}

fn is_drift_code_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx")
    )
}

fn is_drift_code_relative(relative: &str) -> bool {
    matches!(
        Path::new(relative).extension().and_then(|ext| ext.to_str()),
        Some("ts" | "tsx" | "js" | "jsx")
    )
}

fn is_drift_doc_relative(relative: &str) -> bool {
    relative.ends_with(".md")
        && (relative.starts_with("akta-docs/03-business-logic/")
            || relative.starts_with("akta-docs/06-technical/"))
}

fn is_ignored_dir_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".jj"
            | "node_modules"
            | "dist"
            | "build"
            | "target"
            | ".next"
            | ".turbo"
            | ".openakta"
    )
}

#[derive(Debug, Default)]
struct SignatureSnapshot {
    hash: String,
    body: String,
}

fn empty_tree(root: &Path) -> MerkleTree {
    MerkleTree {
        root_path: root.to_path_buf(),
        file_hashes: HashMap::new(),
        block_hashes: HashMap::new(),
    }
}

fn embed_text(content: &str, dim: usize) -> Vec<f32> {
    let mut embedding = vec![0.0f32; dim];
    for (index, byte) in content.bytes().enumerate() {
        embedding[index % dim] += byte as f32 / 255.0;
    }
    let norm = embedding
        .iter()
        .map(|value| value * value)
        .sum::<f32>()
        .sqrt();
    if norm > 0.0 {
        for value in &mut embedding {
            *value /= norm;
        }
    }
    embedding
}

fn decision_tag(decision: &ReconcileDecision) -> &'static str {
    match decision {
        ReconcileDecision::Noop => "noop",
        ReconcileDecision::UpdateRequired => "update_required",
        ReconcileDecision::ReviewRequired => "review_required",
    }
}

fn drift_memory_id(workspace_root: &Path) -> String {
    format!(
        "livingdocs:drift:{}",
        blake3::hash(workspace_root.display().to_string().as_bytes()).to_hex()
    )
}

fn highest_severity_tag(report: &DriftReport) -> &'static str {
    match report.highest_severity {
        Some(openakta_docs::DriftSeverity::Critical) => "critical",
        Some(openakta_docs::DriftSeverity::Warning) => "warning",
        Some(openakta_docs::DriftSeverity::Info) => "info",
        None => "noop",
    }
}

fn render_drift_flags(report: &DriftReport) -> String {
    if report.flags.is_empty() {
        return "No drift detected.".to_string();
    }

    report
        .flags
        .iter()
        .map(|flag| {
            let domain = match flag.domain {
                DriftDomain::ApiSurface => "api_surface",
                DriftDomain::BusinessRule => "business_rule",
                DriftDomain::CodeReference => "code_reference",
            };
            format!(
                "- domain: {domain}\n  kind: {:?}\n  severity: {:?}\n  doc: {}\n  code: {}\n  symbol: {}\n  rules: {}\n  message: {}",
                flag.kind,
                flag.severity,
                flag.doc_path.display(),
                flag.code_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".to_string()),
                flag.symbol_name.as_deref().unwrap_or("<none>"),
                if flag.rule_ids.is_empty() {
                    "<none>".to_string()
                } else {
                    flag.rule_ids.join(",")
                },
                flag.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::{drift_memory_id, LivingDocsProcessor};
    use crate::background::governor::{GovernorConfig, ResourceGovernor};
    use crate::background::queue::{
        JobKind, JobRecord, JobStatus, SqliteJobQueue, SyncJobPayload, SyncScope,
    };
    use crate::background::review_resolution::{
        CodeResolutionRequest, CodeResolutionResult, CodeResolutionRunner,
    };
    use openakta_docs::DriftKind;
    use openakta_memory::PersistentSemanticStore;
    use std::fs;
    use std::sync::Arc;

    #[derive(Clone)]
    struct NoopCodeResolutionRunner;

    impl CodeResolutionRunner for NoopCodeResolutionRunner {
        fn run(&self, _request: &CodeResolutionRequest) -> anyhow::Result<CodeResolutionResult> {
            Ok(CodeResolutionResult {
                patch_receipt_id: "patch-receipt-test".into(),
                summary: "noop".into(),
            })
        }
    }

    fn processor_runner() -> Arc<dyn CodeResolutionRunner> {
        Arc::new(NoopCodeResolutionRunner)
    }

    #[test]
    fn processor_persists_drift_report_for_changed_typescript_file() {
        let root = tempfile::tempdir().expect("tempdir");
        let workspace_root = root.path().to_path_buf();
        fs::create_dir_all(workspace_root.join(".openakta")).expect("openakta dir");
        fs::create_dir_all(workspace_root.join("akta-docs/03-business-logic")).expect("docs dir");
        fs::create_dir_all(workspace_root.join("src/lib")).expect("src dir");

        fs::write(
            workspace_root.join("akta-docs/03-business-logic/rules.md"),
            r#"
CodePath: src/lib/rules.ts
Symbol: resolvePlan
Kind: function
Signature: export function resolvePlan(accountId: string): Promise<string>
RuleID: BR-001
"#,
        )
        .expect("write docs");
        fs::write(
            workspace_root.join("src/lib/rules.ts"),
            r#"
/**
 * @RuleID BR-999
 */
export function resolvePlan(force: boolean): Promise<number> {
  return Promise.resolve(1);
}
"#,
        )
        .expect("write code");

        let semantic_store_path = workspace_root.join(".openakta/semantic.db");
        let queue = SqliteJobQueue::open(workspace_root.join(".openakta/livingdocs-queue.db"))
            .expect("queue");
        let mut processor = LivingDocsProcessor::new(
            workspace_root.clone(),
            semantic_store_path.clone(),
            queue.clone(),
            8 * 1024 * 1024,
            processor_runner(),
        )
        .expect("processor");
        let mut governor = ResourceGovernor::new(GovernorConfig::default()).expect("governor");
        let job = JobRecord {
            id: "job-1".to_string(),
            kind: JobKind::IncrementalSync,
            status: JobStatus::Queued,
            attempts: 0,
            max_attempts: 3,
            payload: SyncJobPayload {
                workspace_root: workspace_root.display().to_string(),
                scope: SyncScope::Paths(vec!["src/lib/rules.ts".to_string()]),
                reason: "test".to_string(),
                event_count: 1,
            },
        };

        processor.process(&job, &mut governor).expect("process");

        let store = PersistentSemanticStore::new(&semantic_store_path, 384).expect("store");
        let memory = store
            .get(&drift_memory_id(&workspace_root))
            .expect("fetch")
            .expect("present");
        let stored_report = queue
            .latest_drift_report(&workspace_root)
            .expect("read report")
            .expect("present");

        assert!(memory.content.contains("flags_total:"));
        assert!(memory
            .content
            .contains("storage: sqlite_primary_semantic_summary"));
        assert!(memory.content.contains("business_rule_flags:"));
        assert!(memory.content.contains("resolvePlan"));
        assert_eq!(stored_report.total_flags, 2);
        assert_eq!(stored_report.business_rule_flags, 1);
    }

    #[test]
    fn processor_refreshes_doc_expectations_after_doc_change() {
        let root = tempfile::tempdir().expect("tempdir");
        let workspace_root = root.path().to_path_buf();
        fs::create_dir_all(workspace_root.join(".openakta")).expect("openakta dir");
        fs::create_dir_all(workspace_root.join("akta-docs/03-business-logic")).expect("docs dir");
        fs::create_dir_all(workspace_root.join("src/lib")).expect("src dir");

        let doc_path = workspace_root.join("akta-docs/03-business-logic/rules.md");
        fs::write(
            &doc_path,
            r#"
```akta-expect
code_path: src/lib/rules.ts
symbol: resolvePlan
kind: function
signature: export function resolvePlan(accountId: string): Promise<string>
rule_ids:
  - BR-001
```
"#,
        )
        .expect("write docs");
        fs::write(
            workspace_root.join("src/lib/rules.ts"),
            r#"
/** @RuleID BR-001 */
export function resolvePlan(accountId: string): Promise<string> {
  return Promise.resolve(accountId);
}
"#,
        )
        .expect("write code");

        let queue = SqliteJobQueue::open(workspace_root.join(".openakta/livingdocs-queue.db"))
            .expect("queue");
        let semantic_store_path = workspace_root.join(".openakta/semantic.db");
        let mut processor = LivingDocsProcessor::new(
            workspace_root.clone(),
            semantic_store_path,
            queue.clone(),
            8 * 1024 * 1024,
            processor_runner(),
        )
        .expect("processor");
        let mut governor = ResourceGovernor::new(GovernorConfig::default()).expect("governor");

        processor
            .process(
                &JobRecord {
                    id: "job-doc-1".to_string(),
                    kind: JobKind::IncrementalSync,
                    status: JobStatus::Queued,
                    attempts: 0,
                    max_attempts: 3,
                    payload: SyncJobPayload {
                        workspace_root: workspace_root.display().to_string(),
                        scope: SyncScope::Paths(vec![
                            "akta-docs/03-business-logic/rules.md".to_string(),
                            "src/lib/rules.ts".to_string(),
                        ]),
                        reason: "initial".to_string(),
                        event_count: 2,
                    },
                },
                &mut governor,
            )
            .expect("initial process");
        assert_eq!(
            queue
                .latest_drift_report(&workspace_root)
                .expect("report")
                .expect("present")
                .total_flags,
            0
        );

        fs::write(
            &doc_path,
            r#"
```akta-expect
code_path: src/lib/rules.ts
symbol: resolveMissing
kind: function
signature: export function resolveMissing(accountId: string): Promise<string>
rule_ids:
  - BR-001
```
"#,
        )
        .expect("rewrite docs");

        processor
            .process(
                &JobRecord {
                    id: "job-doc-2".to_string(),
                    kind: JobKind::IncrementalSync,
                    status: JobStatus::Queued,
                    attempts: 0,
                    max_attempts: 3,
                    payload: SyncJobPayload {
                        workspace_root: workspace_root.display().to_string(),
                        scope: SyncScope::Paths(vec![
                            "akta-docs/03-business-logic/rules.md".to_string()
                        ]),
                        reason: "doc_update".to_string(),
                        event_count: 1,
                    },
                },
                &mut governor,
            )
            .expect("doc refresh");

        let report = queue
            .latest_drift_report(&workspace_root)
            .expect("report")
            .expect("present");
        assert_eq!(report.total_flags, 1);
        assert_eq!(report.flags[0].kind, DriftKind::MissingSymbol);
        assert_eq!(
            report.flags[0].symbol_name.as_deref(),
            Some("resolveMissing")
        );
    }

    #[test]
    fn processor_flags_deleted_code_files_in_latest_report() {
        let root = tempfile::tempdir().expect("tempdir");
        let workspace_root = root.path().to_path_buf();
        fs::create_dir_all(workspace_root.join(".openakta")).expect("openakta dir");
        fs::create_dir_all(workspace_root.join("akta-docs/03-business-logic")).expect("docs dir");
        fs::create_dir_all(workspace_root.join("src/lib")).expect("src dir");

        fs::write(
            workspace_root.join("akta-docs/03-business-logic/rules.md"),
            r#"
CodePath: src/lib/rules.ts
Symbol: resolvePlan
Kind: function
Signature: export function resolvePlan(accountId: string): Promise<string>
RuleID: BR-001
"#,
        )
        .expect("write docs");
        let code_path = workspace_root.join("src/lib/rules.ts");
        fs::write(
            &code_path,
            r#"
/** @RuleID BR-001 */
export function resolvePlan(accountId: string): Promise<string> {
  return Promise.resolve(accountId);
}
"#,
        )
        .expect("write code");

        let queue = SqliteJobQueue::open(workspace_root.join(".openakta/livingdocs-queue.db"))
            .expect("queue");
        let mut processor = LivingDocsProcessor::new(
            workspace_root.clone(),
            workspace_root.join(".openakta/semantic.db"),
            queue.clone(),
            8 * 1024 * 1024,
            processor_runner(),
        )
        .expect("processor");
        let mut governor = ResourceGovernor::new(GovernorConfig::default()).expect("governor");

        processor
            .process(
                &JobRecord {
                    id: "job-delete-1".to_string(),
                    kind: JobKind::IncrementalSync,
                    status: JobStatus::Queued,
                    attempts: 0,
                    max_attempts: 3,
                    payload: SyncJobPayload {
                        workspace_root: workspace_root.display().to_string(),
                        scope: SyncScope::Paths(vec!["src/lib/rules.ts".to_string()]),
                        reason: "initial".to_string(),
                        event_count: 1,
                    },
                },
                &mut governor,
            )
            .expect("initial process");
        assert_eq!(
            queue
                .latest_drift_report(&workspace_root)
                .expect("report")
                .expect("present")
                .total_flags,
            0
        );

        fs::remove_file(&code_path).expect("remove code");
        processor
            .process(
                &JobRecord {
                    id: "job-delete-2".to_string(),
                    kind: JobKind::IncrementalSync,
                    status: JobStatus::Queued,
                    attempts: 0,
                    max_attempts: 3,
                    payload: SyncJobPayload {
                        workspace_root: workspace_root.display().to_string(),
                        scope: SyncScope::Paths(vec!["src/lib/rules.ts".to_string()]),
                        reason: "delete".to_string(),
                        event_count: 1,
                    },
                },
                &mut governor,
            )
            .expect("delete process");

        let report = queue
            .latest_drift_report(&workspace_root)
            .expect("report")
            .expect("present");
        assert_eq!(report.total_flags, 1);
        assert_eq!(report.flags[0].kind, DriftKind::MissingSymbol);
        assert_eq!(report.flags[0].symbol_name.as_deref(), Some("resolvePlan"));
    }

    #[test]
    fn confidence_audit_records_review_required_on_drift() {
        let root = tempfile::tempdir().expect("tempdir");
        let workspace_root = root.path().to_path_buf();
        fs::create_dir_all(workspace_root.join(".openakta")).expect("openakta dir");
        fs::create_dir_all(workspace_root.join("akta-docs/03-business-logic")).expect("docs dir");
        fs::create_dir_all(workspace_root.join("src/lib")).expect("src dir");

        fs::write(
            workspace_root.join("akta-docs/03-business-logic/rules.md"),
            r#"
CodePath: src/lib/rules.ts
Symbol: resolvePlan
Kind: function
Signature: export function resolvePlan(accountId: string): Promise<string>
RuleID: BR-001
"#,
        )
        .expect("write docs");
        fs::write(
            workspace_root.join("src/lib/rules.ts"),
            r#"
/**
 * @RuleID BR-999
 */
export function resolvePlan(force: boolean): Promise<number> {
  return Promise.resolve(1);
}
"#,
        )
        .expect("write code");

        let semantic_store_path = workspace_root.join(".openakta/semantic.db");
        let queue = SqliteJobQueue::open(workspace_root.join(".openakta/livingdocs-queue.db"))
            .expect("queue");
        let mut processor = LivingDocsProcessor::new(
            workspace_root.clone(),
            semantic_store_path,
            queue.clone(),
            8 * 1024 * 1024,
            processor_runner(),
        )
        .expect("processor");
        let mut governor = ResourceGovernor::new(GovernorConfig::default()).expect("governor");
        let job = JobRecord {
            id: "job-audit-review".to_string(),
            kind: JobKind::IncrementalSync,
            status: JobStatus::Queued,
            attempts: 0,
            max_attempts: 3,
            payload: SyncJobPayload {
                workspace_root: workspace_root.display().to_string(),
                scope: SyncScope::Paths(vec!["src/lib/rules.ts".to_string()]),
                reason: "test".to_string(),
                event_count: 1,
            },
        };

        processor.process(&job, &mut governor).expect("process");

        let report = queue
            .latest_drift_report(&workspace_root)
            .expect("read report")
            .expect("present");
        assert!(report.total_flags > 0, "expected drift flags");

        let audit = queue
            .latest_confidence_audit_for_report(&report.report_id)
            .expect("audit query")
            .expect("confidence audit row");
        assert_eq!(audit.decision, "review_required");
        assert!(audit.breakdown_json.contains("after_severity"));
        assert!(audit.canonical_toon_json.is_none());
    }

    #[test]
    fn confidence_audit_records_noop_when_no_drift() {
        let root = tempfile::tempdir().expect("tempdir");
        let workspace_root = root.path().to_path_buf();
        fs::create_dir_all(workspace_root.join(".openakta")).expect("openakta dir");
        fs::create_dir_all(workspace_root.join("akta-docs/03-business-logic")).expect("docs dir");
        fs::create_dir_all(workspace_root.join("src/lib")).expect("src dir");

        let doc_path = workspace_root.join("akta-docs/03-business-logic/rules.md");
        fs::write(
            &doc_path,
            r#"
```akta-expect
code_path: src/lib/rules.ts
symbol: resolvePlan
kind: function
signature: export function resolvePlan(accountId: string): Promise<string>
rule_ids:
  - BR-001
```
"#,
        )
        .expect("write docs");
        fs::write(
            workspace_root.join("src/lib/rules.ts"),
            r#"
/** @RuleID BR-001 */
export function resolvePlan(accountId: string): Promise<string> {
  return Promise.resolve(accountId);
}
"#,
        )
        .expect("write code");

        let queue = SqliteJobQueue::open(workspace_root.join(".openakta/livingdocs-queue.db"))
            .expect("queue");
        let mut processor = LivingDocsProcessor::new(
            workspace_root.clone(),
            workspace_root.join(".openakta/semantic.db"),
            queue.clone(),
            8 * 1024 * 1024,
            processor_runner(),
        )
        .expect("processor");
        let mut governor = ResourceGovernor::new(GovernorConfig::default()).expect("governor");

        processor
            .process(
                &JobRecord {
                    id: "job-audit-noop".to_string(),
                    kind: JobKind::IncrementalSync,
                    status: JobStatus::Queued,
                    attempts: 0,
                    max_attempts: 3,
                    payload: SyncJobPayload {
                        workspace_root: workspace_root.display().to_string(),
                        scope: SyncScope::Paths(vec![
                            "akta-docs/03-business-logic/rules.md".to_string(),
                            "src/lib/rules.ts".to_string(),
                        ]),
                        reason: "aligned".to_string(),
                        event_count: 2,
                    },
                },
                &mut governor,
            )
            .expect("process");

        let report = queue
            .latest_drift_report(&workspace_root)
            .expect("report")
            .expect("present");
        assert_eq!(report.total_flags, 0);

        let audit = queue
            .latest_confidence_audit_for_report(&report.report_id)
            .expect("audit query")
            .expect("confidence audit row");
        assert_eq!(audit.decision, "noop");
        assert_eq!(audit.confidence_score, Some(1.0));
    }
}
