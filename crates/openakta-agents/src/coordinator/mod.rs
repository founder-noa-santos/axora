//! Canonical coordinator surface for OPENAKTA agents.
//!
//! OPENAKTA exposes coordinator v2 as the only supported coordinator runtime.

pub mod v2;

pub use v2::{
    BaseSquadBootstrapper, BlackboardV2, Coordinator, CoordinatorConfig, CoordinatorCore,
    CoordinatorDispatchWorker, CoordinatorDispatchWorkerStatus, CoordinatorDispatcher,
    CoordinatorMetrics, CoordinatorTaskQueue, CoordinatorV2Error, DispatchCompletionReport,
    DispatchLoopReport, MissionResult, MissionStatus, MonitorReport, OutputContract,
    PlanningActingPolicy, RegisteredWorkerInfo, SquadRole, WorkerInfo, WorkerProfile,
    WorkerRegistry,
};
