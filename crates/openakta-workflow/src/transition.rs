//! Story intake / preparation state machines for the local workflow runtime.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoryIntakeStatus {
    Captured,
    Classified,
    ClarificationPending,
    Triaged,
    Preparing,
    Prepared,
    Ready,
    Executing,
    ClosurePending,
    Closed,
    Blocked,
    Abandoned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StoryPreparationStatus {
    Preparing,
    Prepared,
    Ready,
    Executing,
    ClosurePending,
    Closed,
    Blocked,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum MolError {
    #[error("unknown story intake status: {0:?}")]
    UnknownStoryIntakeStatus(String),

    #[error("unknown story preparation status: {0:?}")]
    UnknownStoryPreparationStatus(String),

    #[error("illegal story preparation transition from {from} to {to}")]
    IllegalStoryPreparationTransition {
        from: StoryPreparationStatus,
        to: StoryPreparationStatus,
    },

    #[error("illegal story intake transition from {from} to {to}")]
    IllegalStoryIntakeTransition {
        from: StoryIntakeStatus,
        to: StoryIntakeStatus,
    },

    #[error(
        "legacy work item API must not set MOL-managed fields on prepared-story work items when MOL_STRICT_LEGACY_FENCE is enabled (fields: {fields})"
    )]
    LegacyFenceViolation { fields: String },

    #[error(
        "illegal story intake capture status {status}: cannot start in a terminal or late-phase state"
    )]
    IllegalStoryIntakeCapture { status: StoryIntakeStatus },

    #[error("unknown or inactive persona id: {persona_id}")]
    UnknownPersona { persona_id: String },

    #[error("persona {acting} does not match expected {expected} for {context}")]
    PersonaSubjectMismatch {
        expected: String,
        acting: String,
        context: &'static str,
    },
}

impl StoryIntakeStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Captured => "captured",
            Self::Classified => "classified",
            Self::ClarificationPending => "clarification_pending",
            Self::Triaged => "triaged",
            Self::Preparing => "preparing",
            Self::Prepared => "prepared",
            Self::Ready => "ready",
            Self::Executing => "executing",
            Self::ClosurePending => "closure_pending",
            Self::Closed => "closed",
            Self::Blocked => "blocked",
            Self::Abandoned => "abandoned",
        }
    }
}

impl StoryPreparationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preparing => "preparing",
            Self::Prepared => "prepared",
            Self::Ready => "ready",
            Self::Executing => "executing",
            Self::ClosurePending => "closure_pending",
            Self::Closed => "closed",
            Self::Blocked => "blocked",
        }
    }
}

impl fmt::Display for StoryIntakeStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for StoryPreparationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for StoryIntakeStatus {
    type Err = MolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "captured" => Ok(Self::Captured),
            "classified" => Ok(Self::Classified),
            "clarification_pending" => Ok(Self::ClarificationPending),
            "triaged" => Ok(Self::Triaged),
            "preparing" => Ok(Self::Preparing),
            "prepared" => Ok(Self::Prepared),
            "ready" => Ok(Self::Ready),
            "executing" => Ok(Self::Executing),
            "closure_pending" => Ok(Self::ClosurePending),
            "closed" => Ok(Self::Closed),
            "blocked" => Ok(Self::Blocked),
            "abandoned" => Ok(Self::Abandoned),
            _ => Err(MolError::UnknownStoryIntakeStatus(s.to_string())),
        }
    }
}

impl FromStr for StoryPreparationStatus {
    type Err = MolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "preparing" => Ok(Self::Preparing),
            "prepared" => Ok(Self::Prepared),
            "ready" => Ok(Self::Ready),
            "executing" => Ok(Self::Executing),
            "closure_pending" => Ok(Self::ClosurePending),
            "closed" => Ok(Self::Closed),
            "blocked" => Ok(Self::Blocked),
            _ => Err(MolError::UnknownStoryPreparationStatus(s.to_string())),
        }
    }
}

pub fn validate_story_intake_capture_status(status: &str) -> Result<(), MolError> {
    let s: StoryIntakeStatus = status.parse()?;
    match s {
        StoryIntakeStatus::Closed | StoryIntakeStatus::Abandoned => {
            Err(MolError::IllegalStoryIntakeCapture { status: s })
        }
        StoryIntakeStatus::Prepared
        | StoryIntakeStatus::Ready
        | StoryIntakeStatus::Executing
        | StoryIntakeStatus::ClosurePending => {
            Err(MolError::IllegalStoryIntakeCapture { status: s })
        }
        _ => Ok(()),
    }
}

pub fn validate_story_preparation_capture_status(status: &str) -> Result<(), MolError> {
    let to: StoryPreparationStatus = status.parse()?;
    validate_preparation_transition_enum(StoryPreparationStatus::Preparing, to)
}

pub fn validate_preparation_transition(from: &str, to: &str) -> Result<(), MolError> {
    let from_s: StoryPreparationStatus = from.parse()?;
    let to_s: StoryPreparationStatus = to.parse()?;
    validate_preparation_transition_enum(from_s, to_s)
}

fn validate_preparation_transition_enum(
    from: StoryPreparationStatus,
    to: StoryPreparationStatus,
) -> Result<(), MolError> {
    use StoryPreparationStatus::*;
    if from == to {
        return Ok(());
    }
    if from == Closed {
        return Err(MolError::IllegalStoryPreparationTransition { from, to });
    }
    if to == Closed {
        return if from == ClosurePending {
            Ok(())
        } else {
            Err(MolError::IllegalStoryPreparationTransition { from, to })
        };
    }
    let allowed = matches!(
        (from, to),
        (Preparing, Prepared)
            | (Preparing, Blocked)
            | (Prepared, Ready)
            | (Prepared, Preparing)
            | (Prepared, Blocked)
            | (Ready, Executing)
            | (Ready, Prepared)
            | (Ready, Blocked)
            | (Executing, ClosurePending)
            | (Executing, Blocked)
            | (Executing, Ready)
            | (ClosurePending, Executing)
            | (ClosurePending, Blocked)
            | (ClosurePending, Ready)
            | (Blocked, Preparing)
            | (Blocked, Prepared)
            | (Blocked, Ready)
            | (Blocked, Executing)
            | (Blocked, ClosurePending)
    );
    if allowed {
        Ok(())
    } else {
        Err(MolError::IllegalStoryPreparationTransition { from, to })
    }
}
