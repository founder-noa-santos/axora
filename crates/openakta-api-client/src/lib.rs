//! OPENAKTA API Client SDK
//!
//! This crate provides the client SDK for communicating with the OPENAKTA unified API.
//! It handles connection pooling, retries, circuit breaking, and feature flag routing.

pub mod auth;
pub mod capability;
pub mod client;
pub mod config;
pub mod error;
pub mod execution_strategy;
pub mod feature_flags;
pub mod mol_flags;
pub mod work_management;

mod work_proto_convert;

pub use auth::{static_provider, AuthProvider, EnvAuthProvider, StaticTokenAuthProvider};
pub use capability::{
    Capability, CapabilityNegotiator, NegotiationConstraints, ProviderCapabilities,
};
pub use client::{ApiClient, ApiClientPool};
pub use config::ClientConfig;
pub use error::{ApiError, Result};
pub use execution_strategy::ExecutionStrategy;
pub use feature_flags::FeatureFlags;
pub use mol_flags::MolFeatureFlags;
pub use work_management::{
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

/// Re-export proto types for convenience
pub use openakta_proto::provider_v1;
pub use openakta_proto::research_v1;
pub use openakta_proto::work_v1;
