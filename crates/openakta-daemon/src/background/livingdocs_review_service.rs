//! gRPC [`LivingDocsReviewService`](openakta_proto::livingdocs::v1::living_docs_review_service_server::LivingDocsReviewService) — Plan 6 review queue + SSOT resolution.

use std::path::{Path, PathBuf};
use std::time::Duration;

use openakta_docs::{DriftDomain, DriftKind, DriftSeverity};
use openakta_proto::livingdocs::v1::living_docs_review_service_server::LivingDocsReviewService;
use openakta_proto::livingdocs::v1::{
    GetPendingReviewCountRequest, GetPendingReviewCountResponse, GetReviewDetailRequest,
    GetReviewDetailResponse, ListPendingReviewsRequest, ResolutionOutcome, ReviewDriftFlagView,
    ReviewQueueItem, ReviewQueueListResponse, SsotChoice, SubmitResolutionRequest,
    SubmitResolutionResponse,
};
use tokio::time::{sleep, Instant};
use tonic::{Request, Response, Status};
use tracing::warn;

use crate::background::queue::{
    review_status_code_update_failed, review_status_doc_update_failed,
    review_status_resolved_with_code_update, review_status_resolved_with_doc_update,
    ReconcileReviewRow, SqliteJobQueue, StoredDriftFlag, StoredDriftReport,
    StoredResolutionResult, SubmitResolutionOutcome,
};

/// Binds to the same SQLite queue as [`crate::background::engine::LivingDocsEngine`].
#[derive(Clone, Debug)]
pub struct LivingDocsReviewGrpc {
    queue: SqliteJobQueue,
    /// Only this workspace may call queue RPCs (paths must match this root).
    workspace_root: PathBuf,
    submit_wait_timeout: Duration,
}

impl LivingDocsReviewGrpc {
    pub fn open(queue: SqliteJobQueue, workspace_root: PathBuf) -> Self {
        Self {
            queue,
            workspace_root,
            submit_wait_timeout: Duration::from_secs(20),
        }
    }

    #[cfg(test)]
    fn with_submit_wait_timeout(
        queue: SqliteJobQueue,
        workspace_root: PathBuf,
        submit_wait_timeout: Duration,
    ) -> Self {
        Self {
            queue,
            workspace_root,
            submit_wait_timeout,
        }
    }

    fn resolve_workspace(&self, requested: &str) -> Result<PathBuf, Status> {
        if requested.is_empty() {
            return Ok(self.workspace_root.clone());
        }
        let p = PathBuf::from(requested);
        if paths_equal(&p, &self.workspace_root) {
            return Ok(self.workspace_root.clone());
        }
        Err(Status::permission_denied(
            "workspace_root does not match this daemon workspace",
        ))
    }

    fn row_to_queue_item(
        row: &ReconcileReviewRow,
        primary_doc: Option<&str>,
        highest_severity: Option<&str>,
        summary: Option<&str>,
    ) -> ReviewQueueItem {
        ReviewQueueItem {
            review_id: row.review_id.clone(),
            report_id: row.report_id.clone(),
            workspace_root: row.workspace_root.display().to_string(),
            created_at_ms: row.created_at_ms,
            confidence_score: row.confidence_score,
            primary_doc_path: primary_doc.unwrap_or("").to_string(),
            highest_severity: highest_severity.map(String::from),
            summary: summary.map(String::from),
        }
    }

    fn header_from_review_and_report(
        row: &ReconcileReviewRow,
        report: Option<&StoredDriftReport>,
    ) -> ReviewQueueItem {
        let (primary, high, summary) = match report {
            Some(rep) => {
                let primary = rep
                    .flags
                    .first()
                    .map(|f| f.doc_path.display().to_string())
                    .unwrap_or_default();
                let high = rep
                    .highest_severity
                    .as_ref()
                    .map(severity_to_label);
                let summary = rep.flags.first().map(|f| f.message.clone());
                (primary, high, summary)
            }
            None => (String::new(), None, None),
        };
        ReviewQueueItem {
            review_id: row.review_id.clone(),
            report_id: row.report_id.clone(),
            workspace_root: row.workspace_root.display().to_string(),
            created_at_ms: row.created_at_ms,
            confidence_score: row.confidence_score,
            primary_doc_path: primary,
            highest_severity: high,
            summary,
        }
    }

    fn flag_to_view(workspace_root: &Path, f: &StoredDriftFlag) -> ReviewDriftFlagView {
        let (expected_excerpt, actual_excerpt) = derive_flag_excerpts(workspace_root, f);
        ReviewDriftFlagView {
            fingerprint: f.fingerprint.clone(),
            domain: domain_to_label(&f.domain),
            kind: kind_to_label(&f.kind),
            severity: severity_to_label(&f.severity),
            doc_path: f.doc_path.display().to_string(),
            code_path: f.code_path.as_ref().map(|p| p.display().to_string()),
            symbol_name: f.symbol_name.clone(),
            rule_ids: f.rule_ids.clone(),
            message: f.message.clone(),
            expected_excerpt,
            actual_excerpt,
        }
    }

    async fn wait_for_terminal_resolution(
        &self,
        review_id: &str,
    ) -> Result<Option<StoredResolutionResult>, Status> {
        let deadline = Instant::now() + self.submit_wait_timeout;
        loop {
            let snapshot = self
                .queue
                .resolution_result(review_id)
                .map_err(|e| Status::internal(e.to_string()))?;
            let Some(snapshot) = snapshot else {
                return Ok(None);
            };
            if is_terminal_review_status(&snapshot.status) {
                return Ok(Some(snapshot));
            }
            if Instant::now() >= deadline {
                return Ok(Some(snapshot));
            }
            sleep(Duration::from_millis(100)).await;
        }
    }
}

#[tonic::async_trait]
impl LivingDocsReviewService for LivingDocsReviewGrpc {
    async fn list_pending_reviews(
        &self,
        request: Request<ListPendingReviewsRequest>,
    ) -> Result<Response<ReviewQueueListResponse>, Status> {
        let req = request.into_inner();
        let ws = self.resolve_workspace(&req.workspace_root)?;
        let page_size = if req.page_size == 0 {
            50
        } else {
            req.page_size
        };
        let rows = self
            .queue
            .list_pending_reviews(&ws, page_size, req.page_offset)
            .map_err(|e| Status::internal(e.to_string()))?;
        let total = self
            .queue
            .pending_review_count(&ws)
            .map_err(|e| Status::internal(e.to_string()))?;
        let items = rows
            .into_iter()
            .map(|(row, primary, high, summary)| {
                Self::row_to_queue_item(
                    &row,
                    primary.as_deref(),
                    high.as_deref(),
                    summary.as_deref(),
                )
            })
            .collect();
        Ok(Response::new(ReviewQueueListResponse {
            items,
            total_pending: total,
        }))
    }

    async fn get_pending_review_count(
        &self,
        request: Request<GetPendingReviewCountRequest>,
    ) -> Result<Response<GetPendingReviewCountResponse>, Status> {
        let req = request.into_inner();
        let ws = self.resolve_workspace(&req.workspace_root)?;
        let count = self
            .queue
            .pending_review_count(&ws)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(GetPendingReviewCountResponse { count }))
    }

    async fn get_review_detail(
        &self,
        request: Request<GetReviewDetailRequest>,
    ) -> Result<Response<GetReviewDetailResponse>, Status> {
        let req = request.into_inner();
        if req.review_id.is_empty() {
            return Err(Status::invalid_argument("review_id is required"));
        }
        let row = self
            .queue
            .reconcile_review_by_id(&req.review_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        let Some(row) = row else {
            return Err(Status::not_found("unknown review_id"));
        };
        if !paths_equal(&row.workspace_root, &self.workspace_root) {
            return Err(Status::permission_denied("review belongs to another workspace"));
        }
        let report = self
            .queue
            .drift_report_by_report_id(&row.report_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        let header = Self::header_from_review_and_report(&row, report.as_ref());
        let flags: Vec<ReviewDriftFlagView> = report
            .as_ref()
            .map(|r| {
                r.flags
                    .iter()
                    .map(|flag| Self::flag_to_view(&self.workspace_root, flag))
                    .collect()
            })
            .unwrap_or_default();
        let audit = self
            .queue
            .latest_confidence_audit_action_id(&row.report_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        Ok(Response::new(GetReviewDetailResponse {
            header: Some(header),
            flags,
            breakdown_json: row.breakdown_json,
            confidence_audit_action_id: audit,
        }))
    }

    async fn submit_resolution(
        &self,
        request: Request<SubmitResolutionRequest>,
    ) -> Result<Response<SubmitResolutionResponse>, Status> {
        let req = request.into_inner();
        if req.review_id.is_empty() {
            return Err(Status::invalid_argument("review_id is required"));
        }
        if req.client_resolution_id.is_empty() {
            return Err(Status::invalid_argument("client_resolution_id is required"));
        }
        let choice = SsotChoice::try_from(req.choice).unwrap_or(SsotChoice::Unspecified);
        let ssot_label = match choice {
            SsotChoice::UpdateDoc => "update_doc",
            SsotChoice::UpdateCode => "update_code",
            SsotChoice::Unspecified => {
                return Ok(Response::new(SubmitResolutionResponse {
                    server_resolution_id: String::new(),
                    outcome: ResolutionOutcome::Rejected as i32,
                    patch_receipt_id: None,
                    toon_changelog_entry_id: None,
                }));
            }
        };

        let row = self
            .queue
            .reconcile_review_by_id(&req.review_id)
            .map_err(|e| Status::internal(e.to_string()))?;
        let Some(row) = row else {
            return Ok(Response::new(SubmitResolutionResponse {
                server_resolution_id: String::new(),
                outcome: ResolutionOutcome::Rejected as i32,
                patch_receipt_id: None,
                toon_changelog_entry_id: None,
            }));
        };
        if !paths_equal(&row.workspace_root, &self.workspace_root) {
            return Err(Status::permission_denied("review belongs to another workspace"));
        }

        let note = req.user_note.as_deref();
        let (server_resolution_id, duplicate) = match self
            .queue
            .submit_resolution(
                &req.review_id,
                ssot_label,
                &req.client_resolution_id,
                note,
            )
            .map_err(|e| Status::internal(e.to_string()))?
        {
            SubmitResolutionOutcome::Ok {
                server_resolution_id,
            } => (server_resolution_id, false),
            SubmitResolutionOutcome::Duplicate {
                server_resolution_id,
            } => (server_resolution_id, true),
            SubmitResolutionOutcome::NotFound => {
                return Ok(Response::new(SubmitResolutionResponse {
                    server_resolution_id: String::new(),
                    outcome: ResolutionOutcome::Rejected as i32,
                    patch_receipt_id: None,
                    toon_changelog_entry_id: None,
                }));
            }
            SubmitResolutionOutcome::Conflict { reason } => {
                warn!(%reason, "submit_resolution conflict");
                return Ok(Response::new(SubmitResolutionResponse {
                    server_resolution_id: String::new(),
                    outcome: ResolutionOutcome::Conflict as i32,
                    patch_receipt_id: None,
                    toon_changelog_entry_id: None,
                }));
            }
        };

        let snapshot = self.wait_for_terminal_resolution(&req.review_id).await?;
        let Some(snapshot) = snapshot else {
            return Ok(Response::new(SubmitResolutionResponse {
                server_resolution_id,
                outcome: ResolutionOutcome::InternalError as i32,
                patch_receipt_id: None,
                toon_changelog_entry_id: None,
            }));
        };
        let outcome = if duplicate {
            ResolutionOutcome::Duplicate
        } else {
            ResolutionOutcome::Ok
        };
        match snapshot.status.as_str() {
            status if status == review_status_resolved_with_doc_update() => {
                Ok(Response::new(SubmitResolutionResponse {
                    server_resolution_id,
                    outcome: outcome as i32,
                    patch_receipt_id: snapshot.patch_receipt_id,
                    toon_changelog_entry_id: snapshot.toon_changelog_entry_id,
                }))
            }
            status if status == review_status_resolved_with_code_update() => {
                Ok(Response::new(SubmitResolutionResponse {
                    server_resolution_id,
                    outcome: outcome as i32,
                    patch_receipt_id: snapshot.patch_receipt_id,
                    toon_changelog_entry_id: snapshot.toon_changelog_entry_id,
                }))
            }
            status
                if status == review_status_doc_update_failed()
                    || status == review_status_code_update_failed() =>
            {
                warn!(
                    review_id = %req.review_id,
                    server_resolution_id = %server_resolution_id,
                    status,
                    error = ?snapshot.resolution_error,
                    "submit_resolution follow-up failed"
                );
                Ok(Response::new(SubmitResolutionResponse {
                    server_resolution_id,
                    outcome: ResolutionOutcome::InternalError as i32,
                    patch_receipt_id: snapshot.patch_receipt_id,
                    toon_changelog_entry_id: snapshot.toon_changelog_entry_id,
                }))
            }
            _ => Ok(Response::new(SubmitResolutionResponse {
                server_resolution_id,
                outcome: ResolutionOutcome::InternalError as i32,
                patch_receipt_id: snapshot.patch_receipt_id,
                toon_changelog_entry_id: snapshot.toon_changelog_entry_id,
            })),
        }
    }
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    normalize_workspace_path(a) == normalize_workspace_path(b)
}

fn normalize_workspace_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| {
        if path.is_absolute() {
            path.components().collect()
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
                .components()
                .collect()
        }
    })
}

fn is_terminal_review_status(status: &str) -> bool {
    matches!(
        status,
        "resolved_with_doc_update"
            | "resolved_with_code_update"
            | "doc_update_failed"
            | "code_update_failed"
            | "rejected"
            | "superseded"
            | "approved"
    )
}

fn derive_flag_excerpts(workspace_root: &Path, flag: &StoredDriftFlag) -> (String, String) {
    let expected = read_excerpt_from_file(&flag.doc_path, excerpt_needles(flag, true))
        .unwrap_or_else(|| flag.message.clone());
    let actual = flag
        .code_path
        .as_ref()
        .and_then(|path| read_excerpt_from_file(path, excerpt_needles(flag, false)))
        .unwrap_or_else(|| {
            if let Some(code_path) = &flag.code_path {
                code_path
                    .strip_prefix(workspace_root)
                    .unwrap_or(code_path.as_path())
                    .display()
                    .to_string()
            } else {
                flag.message.clone()
            }
        });
    (expected, actual)
}

fn excerpt_needles(flag: &StoredDriftFlag, expected: bool) -> Vec<String> {
    let mut needles = Vec::new();
    if let Some(symbol_name) = &flag.symbol_name {
        needles.push(symbol_name.clone());
    }
    needles.extend(flag.rule_ids.iter().cloned());
    if expected {
        needles.push(flag.doc_path.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_string());
    } else if let Some(code_path) = &flag.code_path {
        needles.push(
            code_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default()
                .to_string(),
        );
    }
    needles.retain(|needle| !needle.is_empty());
    needles
}

fn read_excerpt_from_file(path: &Path, needles: Vec<String>) -> Option<String> {
    let content = std::fs::read_to_string(path).ok()?;
    let lines = content.lines().collect::<Vec<_>>();
    let match_index = if needles.is_empty() {
        0
    } else {
        lines.iter().position(|line| needles.iter().any(|needle| line.contains(needle)))?
    };
    let start = match_index.saturating_sub(2);
    let end = usize::min(match_index + 3, lines.len());
    Some(lines[start..end].join("\n"))
}

fn domain_to_label(d: &DriftDomain) -> String {
    match d {
        DriftDomain::ApiSurface => "api_surface".into(),
        DriftDomain::BusinessRule => "business_rule".into(),
        DriftDomain::CodeReference => "code_reference".into(),
    }
}

fn kind_to_label(k: &DriftKind) -> String {
    match k {
        DriftKind::MissingSymbol => "missing_symbol".into(),
        DriftKind::SignatureMismatch => "signature_mismatch".into(),
        DriftKind::MissingRuleBinding => "missing_rule_binding".into(),
        DriftKind::StructuralDrift => "structural_drift".into(),
        DriftKind::DeadCodeReference => "dead_code_reference".into(),
    }
}

fn severity_to_label(s: &DriftSeverity) -> String {
    match s {
        DriftSeverity::Critical => "critical".into(),
        DriftSeverity::Warning => "warning".into(),
        DriftSeverity::Info => "info".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openakta_docs::{DriftDomain, DriftKind, DriftReport, DriftSeverity, InconsistencyFlag};
    use openakta_proto::livingdocs::v1::living_docs_review_service_client::LivingDocsReviewServiceClient;
    use openakta_proto::livingdocs::v1::living_docs_review_service_server::LivingDocsReviewService;
    use openakta_proto::livingdocs::v1::{
        GetPendingReviewCountRequest, GetReviewDetailRequest, ListPendingReviewsRequest,
        ResolutionOutcome, SsotChoice, SubmitResolutionRequest,
    };
    use tonic::{Code, Request};
    use tokio::net::TcpListener;
    use tokio_stream::wrappers::TcpListenerStream;

    fn seed_queue_with_pending_review(
        dir: &tempfile::TempDir,
    ) -> (SqliteJobQueue, PathBuf, String, String) {
        let workspace_root = dir.path().join("workspace");
        std::fs::create_dir_all(workspace_root.join("akta-docs")).unwrap();
        std::fs::create_dir_all(workspace_root.join("src")).unwrap();
        std::fs::write(
            workspace_root.join("akta-docs/rules.md"),
            "Rule BR-9\nDocumented symbol alpha must exist.\n",
        )
        .unwrap();
        std::fs::write(
            workspace_root.join("src/x.ts"),
            "export function alpha(): void {}\n",
        )
        .unwrap();
        let queue = SqliteJobQueue::open(dir.path().join("queue.db")).unwrap();
        let report = DriftReport {
            total_flags: 1,
            api_surface_flags: 1,
            business_rule_flags: 0,
            code_reference_flags: 0,
            critical_flags: 1,
            warning_flags: 0,
            info_flags: 0,
            highest_severity: Some(DriftSeverity::Critical),
            flags: vec![InconsistencyFlag {
                domain: DriftDomain::ApiSurface,
                kind: DriftKind::MissingSymbol,
                severity: DriftSeverity::Critical,
                message: "documented symbol missing".into(),
                doc_path: workspace_root.join("akta-docs/rules.md"),
                code_path: Some(workspace_root.join("src/x.ts")),
                symbol_name: Some("alpha".into()),
                rule_ids: vec!["BR-9".into()],
                fingerprint: "fp-grpc".into(),
            }],
        };
        let report_id = queue
            .persist_drift_report(&workspace_root, "incremental_sync", &report)
            .unwrap()
            .0;
        let review_id = queue
            .enqueue_reconcile_review(
                &report_id,
                &workspace_root,
                None,
                0.33,
                r#"{"route":"review_required"}"#,
                "review_required",
            )
            .unwrap()
            .expect("enqueue review");
        (queue, workspace_root, report_id, review_id)
    }

    #[tokio::test]
    async fn grpc_list_and_count_match_pending_rows() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, report_id, _review_id) = seed_queue_with_pending_review(&dir);
        let svc = LivingDocsReviewGrpc::open(queue.clone(), workspace_root.clone());
        let ws = workspace_root.display().to_string();

        let list = svc
            .list_pending_reviews(Request::new(ListPendingReviewsRequest {
                workspace_root: ws.clone(),
                page_size: 20,
                page_offset: 0,
            }))
            .await
            .expect("list")
            .into_inner();
        assert_eq!(list.total_pending, 1);
        assert_eq!(list.items.len(), 1);
        assert_eq!(list.items[0].report_id, report_id);
        assert!(list.items[0].primary_doc_path.contains("rules.md"));

        let count = svc
            .get_pending_review_count(Request::new(GetPendingReviewCountRequest {
                workspace_root: ws,
            }))
            .await
            .expect("count")
            .into_inner();
        assert_eq!(count.count, 1);
    }

    #[tokio::test]
    async fn grpc_empty_workspace_root_uses_daemon_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, _, _) = seed_queue_with_pending_review(&dir);
        let svc = LivingDocsReviewGrpc::open(queue, workspace_root.clone());

        let count = svc
            .get_pending_review_count(Request::new(GetPendingReviewCountRequest {
                workspace_root: String::new(),
            }))
            .await
            .expect("count")
            .into_inner();
        assert_eq!(count.count, 1);
    }

    #[tokio::test]
    async fn grpc_rejects_foreign_workspace() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, _, _) = seed_queue_with_pending_review(&dir);
        let svc = LivingDocsReviewGrpc::open(queue, workspace_root);

        let err = svc
            .list_pending_reviews(Request::new(ListPendingReviewsRequest {
                workspace_root: "/nope/not/our/workspace".into(),
                page_size: 10,
                page_offset: 0,
            }))
            .await
            .err()
            .expect("permission denied");
        assert_eq!(err.code(), Code::PermissionDenied);
    }

    #[tokio::test]
    async fn grpc_get_review_detail_returns_flags_and_breakdown() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, _report_id, review_id) = seed_queue_with_pending_review(&dir);
        let svc = LivingDocsReviewGrpc::open(queue, workspace_root);

        let detail = svc
            .get_review_detail(Request::new(GetReviewDetailRequest {
                review_id: review_id.clone(),
            }))
            .await
            .expect("detail")
            .into_inner();
        let header = detail.header.expect("header");
        assert_eq!(header.review_id, review_id);
        assert_eq!(detail.flags.len(), 1);
        assert_eq!(detail.flags[0].domain, "api_surface");
        assert_eq!(detail.flags[0].kind, "missing_symbol");
        assert_eq!(detail.flags[0].rule_ids, vec!["BR-9"]);
        assert_eq!(detail.breakdown_json, r#"{"route":"review_required"}"#);
        assert!(detail.flags[0].expected_excerpt.contains("BR-9"));
        assert!(detail.flags[0].actual_excerpt.contains("alpha"));
    }

    #[tokio::test]
    async fn grpc_get_review_detail_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, _, _) = seed_queue_with_pending_review(&dir);
        let svc = LivingDocsReviewGrpc::open(queue, workspace_root);

        let err = svc
            .get_review_detail(Request::new(GetReviewDetailRequest {
                review_id: "missing-review".into(),
            }))
            .await
            .err()
            .expect("not found");
        assert_eq!(err.code(), Code::NotFound);
    }

    #[tokio::test]
    async fn grpc_submit_resolution_happy_path_and_rejects_unspecified_choice() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, _, review_id) = seed_queue_with_pending_review(&dir);
        let svc = LivingDocsReviewGrpc::with_submit_wait_timeout(
            queue.clone(),
            workspace_root,
            Duration::from_secs(2),
        );

        let rejected = svc
            .submit_resolution(Request::new(SubmitResolutionRequest {
                review_id: review_id.clone(),
                choice: SsotChoice::Unspecified as i32,
                client_resolution_id: "u1".into(),
                user_note: None,
            }))
            .await
            .expect("ok response")
            .into_inner();
        assert_eq!(rejected.outcome, ResolutionOutcome::Rejected as i32);

        let queue_for_complete = queue.clone();
        let review_for_complete = review_id.clone();
        tokio::spawn(async move {
            for _ in 0..20 {
                if let Ok(Some(work)) =
                    queue_for_complete.claim_next_resolution_work(std::time::Duration::from_secs(
                        30 * 60,
                    ))
                {
                    queue_for_complete
                        .complete_resolution(
                            &review_for_complete,
                            review_status_resolved_with_doc_update(),
                            None,
                            Some("toon-test-id"),
                            None,
                        )
                        .unwrap();
                    assert_eq!(work.choice, "update_doc");
                    return;
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            panic!("resolution work was not claimed");
        });

        let ok = svc
            .submit_resolution(Request::new(SubmitResolutionRequest {
                review_id: review_id.clone(),
                choice: SsotChoice::UpdateDoc as i32,
                client_resolution_id: "u2".into(),
                user_note: Some("note".into()),
            }))
            .await
            .expect("submit")
            .into_inner();
        assert_eq!(ok.outcome, ResolutionOutcome::Ok as i32);
        assert!(!ok.server_resolution_id.is_empty());
        assert_eq!(ok.toon_changelog_entry_id.as_deref(), Some("toon-test-id"));

        let dup = svc
            .submit_resolution(Request::new(SubmitResolutionRequest {
                review_id: review_id.clone(),
                choice: SsotChoice::UpdateCode as i32,
                client_resolution_id: "u2".into(),
                user_note: None,
            }))
            .await
            .expect("dup")
            .into_inner();
        assert_eq!(dup.outcome, ResolutionOutcome::Duplicate as i32);
    }

    #[tokio::test]
    async fn grpc_accepts_canonicalized_workspace_paths() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, _, _) = seed_queue_with_pending_review(&dir);
        let aliased_root = dir.path().join("workspace/.");
        let svc = LivingDocsReviewGrpc::open(queue, workspace_root);

        let count = svc
            .get_pending_review_count(Request::new(GetPendingReviewCountRequest {
                workspace_root: aliased_root.display().to_string(),
            }))
            .await
            .expect("count")
            .into_inner();
        assert_eq!(count.count, 1);
    }

    #[tokio::test]
    async fn grpc_tonic_server_round_trip_lists_reviews() {
        let dir = tempfile::tempdir().unwrap();
        let (queue, workspace_root, _, review_id) = seed_queue_with_pending_review(&dir);
        let svc = LivingDocsReviewGrpc::open(queue, workspace_root);
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let incoming = TcpListenerStream::new(listener);
        let server = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(
                    openakta_proto::livingdocs::v1::living_docs_review_service_server::LivingDocsReviewServiceServer::new(svc),
                )
                .serve_with_incoming(incoming)
                .await
                .unwrap();
        });

        let endpoint = format!("http://{addr}");
        let mut client = LivingDocsReviewServiceClient::connect(endpoint)
            .await
            .expect("client");
        let list = client
            .list_pending_reviews(ListPendingReviewsRequest {
                workspace_root: String::new(),
                page_size: 10,
                page_offset: 0,
            })
            .await
            .expect("list")
            .into_inner();
        assert_eq!(list.total_pending, 1);
        assert_eq!(list.items[0].review_id, review_id);

        server.abort();
    }
}
