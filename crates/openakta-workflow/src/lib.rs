//! Local workflow policy and domain helpers.
//!
//! This crate is the local runtime home for workflow semantics. It may depend on shared
//! workflow contracts during the brownfield migration, but cloud services must not depend on it.

pub mod closure_engine;
pub mod legacy_fence;
pub mod mol_flags;
pub mod transition;

pub use closure_engine::{
    dedup_ids, evaluate_closure, evaluate_closure_with_mol, ClosureEngineError, ClosureSnapshot,
};
pub use legacy_fence::{check_legacy_create_work_item, check_legacy_patch_work_item};
pub use mol_flags::MolFeatureFlags;
pub use openakta_api_client::work_management::{
    AcceptanceCheckUpsertItem, AcceptanceCheckView, ClarificationItemView, ClosureClaimView,
    ClosureGateView, ClosureReportView, CommandEnvelope, CommandResponse, CyclePhaseView,
    DecisionRecordView, DeleteAcceptanceCheckPayload, DependencyEdgeView, EventsResponse,
    EvidenceLinkView, ExecutionProfileDecisionView, HandoffContractView, KnowledgeArtifactView,
    MemoryPromotionEventView, PersonaAssignmentView, PersonaAssignmentsListView, PersonaView,
    PlanVersionView, PlanningCycleView, ReadModelResponse, RecordClarificationAnswerItem,
    RecordClarificationAnswersPayload, RequirementCoverageView, RequirementEdgeView,
    RequirementGraphView, RequirementView, StoryIntakeView, StoryPreparationView,
    UpsertAcceptanceChecksPayload, VerificationFindingView, VerificationRunView, WorkEvent,
    WorkItemView, WorkspaceView,
};
pub use transition::{
    validate_preparation_transition, validate_story_intake_capture_status,
    validate_story_preparation_capture_status, MolError, StoryIntakeStatus, StoryPreparationStatus,
};
