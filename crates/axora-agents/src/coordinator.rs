//! Canonical coordinator surface for AXORA agents.
//!
//! AXORA no longer keeps a separate legacy coordinator runtime. This module
//! exposes CoordinatorV2 as the only supported coordinator implementation.

pub mod v2;

pub use v2::{
    BaseSquadBootstrapper, BlackboardV2, Coordinator, CoordinatorConfig, CoordinatorCore,
    CoordinatorDispatchWorker,
    CoordinatorDispatchWorkerStatus, CoordinatorDispatcher, CoordinatorMetrics,
    CoordinatorTaskQueue, CoordinatorV2Error, DispatchCompletionReport, DispatchLoopReport,
    MissionResult, MissionStatus, MonitorReport, OutputContract, PlanningActingPolicy,
    RegisteredWorkerInfo, SquadRole, WorkerInfo, WorkerProfile, WorkerRegistry,
};
