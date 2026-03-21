//! Conflict resolution for multi-agent disagreements

use crate::agent::Agent;
use crate::error::AgentError;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Conflict between agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Conflict ID
    pub id: String,
    /// Agent A proposal
    pub proposal_a: String,
    /// Agent B proposal
    pub proposal_b: String,
    /// Conflict type
    pub conflict_type: ConflictType,
    /// Status
    pub status: ConflictStatus,
}

/// Type of conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictType {
    /// Different implementations suggested
    ImplementationDifference,
    /// Architecture disagreement
    ArchitectureDisagreement,
    /// Resource contention
    ResourceContention,
    /// Code review rejection
    ReviewRejection,
    /// Other
    Other(String),
}

/// Conflict status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConflictStatus {
    /// Conflict detected, awaiting resolution
    Pending,
    /// Being resolved
    InProgress,
    /// Resolved
    Resolved,
    /// Escalated to human
    Escalated,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Voting with threshold
    Voting { threshold: f32 },
    /// Designated arbiter decides
    Arbitration { arbiter_id: String },
    /// Merge both proposals
    Merge { merger_id: String },
    /// Escalate to human
    HumanEscalation,
}

/// Conflict resolver
pub struct ConflictResolver {
    /// Default resolution strategy
    default_strategy: ConflictResolution,
    /// Resolved conflicts history
    history: Vec<Conflict>,
}

impl ConflictResolver {
    /// Create new conflict resolver
    pub fn new(strategy: ConflictResolution) -> Self {
        Self {
            default_strategy: strategy,
            history: Vec::new(),
        }
    }

    /// Resolve conflict
    pub async fn resolve(
        &mut self,
        conflict: &Conflict,
        agents: &[Arc<dyn Agent>],
    ) -> Result<Resolution> {
        info!("Resolving conflict: {}", conflict.id);

        match &self.default_strategy {
            ConflictResolution::Voting { threshold } => {
                self.resolve_by_voting(conflict, agents, *threshold).await
            }
            ConflictResolution::Arbitration { arbiter_id } => {
                self.resolve_by_arbitration(conflict, agents, arbiter_id)
                    .await
            }
            ConflictResolution::Merge { merger_id } => {
                self.resolve_by_merge(conflict, agents, merger_id).await
            }
            ConflictResolution::HumanEscalation => self.resolve_by_escalation(conflict).await,
        }
    }

    /// Resolve by voting
    async fn resolve_by_voting(
        &self,
        conflict: &Conflict,
        agents: &[Arc<dyn Agent>],
        threshold: f32,
    ) -> Result<Resolution> {
        debug!("Resolving by voting (threshold: {})", threshold);

        let mut votes_a = 0;
        let mut votes_b = 0;

        // Each agent votes
        for agent in agents {
            // In production, agents would evaluate both proposals
            // For now, simulate voting
            let vote = if agent.id() < "mid" {
                votes_a += 1;
                Vote::ForA
            } else {
                votes_b += 1;
                Vote::ForB
            };

            debug!("Agent {} votes: {:?}", agent.id(), vote);
        }

        let total = votes_a + votes_b;
        if total == 0 {
            return Ok(Resolution {
                conflict_id: conflict.id.clone(),
                decision: Decision::Escalate,
                rationale: "No agents were available to vote".to_string(),
            });
        }
        let ratio_a = votes_a as f32 / total as f32;

        let resolution = if ratio_a >= threshold {
            Resolution {
                conflict_id: conflict.id.clone(),
                decision: Decision::ChooseA(conflict.proposal_a.clone()),
                rationale: format!("Proposal A won with {}/{} votes", votes_a, total),
            }
        } else if (1.0 - ratio_a) >= threshold {
            Resolution {
                conflict_id: conflict.id.clone(),
                decision: Decision::ChooseB(conflict.proposal_b.clone()),
                rationale: format!("Proposal B won with {}/{} votes", votes_b, total),
            }
        } else {
            // No clear winner, escalate
            Resolution {
                conflict_id: conflict.id.clone(),
                decision: Decision::Escalate,
                rationale: format!("No clear winner (A: {}, B: {})", votes_a, votes_b),
            }
        };

        Ok(resolution)
    }

    /// Resolve by arbitration
    async fn resolve_by_arbitration(
        &self,
        conflict: &Conflict,
        agents: &[Arc<dyn Agent>],
        arbiter_id: &str,
    ) -> Result<Resolution> {
        debug!("Resolving by arbitration (arbiter: {})", arbiter_id);

        // Find arbiter agent
        let arbiter = agents
            .iter()
            .find(|a| a.id() == arbiter_id)
            .ok_or_else(|| AgentError::AgentNotFound(arbiter_id.to_string()))?;

        let review_task = crate::task::Task::new(&format!(
            "Arbitrate between proposal A and proposal B.\nA:\n{}\n\nB:\n{}",
            conflict.proposal_a, conflict.proposal_b
        ))
        .with_task_type(crate::task::TaskType::Review);
        let review = {
            let mut arbiter = Arc::clone(arbiter);
            Arc::get_mut(&mut arbiter)
                .ok_or_else(|| {
                    AgentError::ExecutionFailed(
                        "arbiter agent is shared and cannot execute mutably".to_string(),
                    )
                })?
                .execute(review_task)?
        };
        let normalized = review.output.to_ascii_lowercase();
        let decision = if normalized.contains("proposal b") || normalized.contains("choose b") {
            Decision::ChooseB(conflict.proposal_b.clone())
        } else {
            Decision::ChooseA(conflict.proposal_a.clone())
        };

        let resolution = Resolution {
            conflict_id: conflict.id.clone(),
            decision,
            rationale: format!("Arbiter {} made the decision", arbiter.name()),
        };

        Ok(resolution)
    }

    /// Resolve by merge
    async fn resolve_by_merge(
        &self,
        conflict: &Conflict,
        agents: &[Arc<dyn Agent>],
        merger_id: &str,
    ) -> Result<Resolution> {
        debug!("Resolving by merge (merger: {})", merger_id);

        // Find merger agent
        let _merger = agents
            .iter()
            .find(|a| a.id() == merger_id)
            .ok_or_else(|| AgentError::AgentNotFound(merger_id.to_string()))?;

        // Merger combines both proposals
        // In production, merger would actually merge the proposals
        let merged = format!(
            "Merged: {}\nAND\n{}",
            conflict.proposal_a, conflict.proposal_b
        );

        let resolution = Resolution {
            conflict_id: conflict.id.clone(),
            decision: Decision::Merge(merged),
            rationale: format!("Merger {} combined both proposals", merger_id),
        };

        Ok(resolution)
    }

    /// Resolve by escalation
    async fn resolve_by_escalation(&mut self, conflict: &Conflict) -> Result<Resolution> {
        warn!("Escalating conflict {} to human", conflict.id);

        let resolution = Resolution {
            conflict_id: conflict.id.clone(),
            decision: Decision::Escalate,
            rationale: "Requires human intervention".to_string(),
        };

        // Mark as escalated
        let mut escalated_conflict = conflict.clone();
        escalated_conflict.status = ConflictStatus::Escalated;
        self.history.push(escalated_conflict);

        Ok(resolution)
    }

    /// Get conflict history
    pub fn history(&self) -> &[Conflict] {
        &self.history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.history.clear();
    }
}

impl Default for ConflictResolver {
    fn default() -> Self {
        Self::new(ConflictResolution::Arbitration {
            arbiter_id: "senior-reviewer".to_string(),
        })
    }
}

/// Vote in conflict resolution
#[derive(Debug, Clone)]
pub enum Vote {
    /// Vote for proposal A
    ForA,
    /// Vote for proposal B
    ForB,
    /// Abstain
    Abstain,
}

/// Resolution of a conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    /// Conflict ID
    pub conflict_id: String,
    /// Decision made
    pub decision: Decision,
    /// Rationale for decision
    pub rationale: String,
}

/// Decision made in conflict resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Decision {
    /// Choose proposal A
    ChooseA(String),
    /// Choose proposal B
    ChooseB(String),
    /// Merge both proposals
    Merge(String),
    /// Escalate to human
    Escalate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_voting_resolution() {
        let mut resolver = ConflictResolver::new(ConflictResolution::Voting { threshold: 0.6 });

        let conflict = Conflict {
            id: "test1".to_string(),
            proposal_a: "Implementation A".to_string(),
            proposal_b: "Implementation B".to_string(),
            conflict_type: ConflictType::ImplementationDifference,
            status: ConflictStatus::Pending,
        };

        let agents: Vec<Arc<dyn Agent>> = Vec::new();
        let resolution = resolver.resolve(&conflict, &agents).await.unwrap();

        assert_eq!(resolution.conflict_id, "test1");
        assert!(matches!(resolution.decision, Decision::Escalate)); // No voters
    }

    #[tokio::test]
    async fn test_arbitration_resolution() {
        let mut resolver = ConflictResolver::new(ConflictResolution::Arbitration {
            arbiter_id: "arbiter".to_string(),
        });

        let conflict = Conflict {
            id: "test2".to_string(),
            proposal_a: "Implementation A".to_string(),
            proposal_b: "Implementation B".to_string(),
            conflict_type: ConflictType::ImplementationDifference,
            status: ConflictStatus::Pending,
        };

        let agents: Vec<Arc<dyn Agent>> = Vec::new();

        // Should fail because arbiter not found
        let result = resolver.resolve(&conflict, &agents).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_conflict_creation() {
        let conflict = Conflict {
            id: "test3".to_string(),
            proposal_a: "A".to_string(),
            proposal_b: "B".to_string(),
            conflict_type: ConflictType::ReviewRejection,
            status: ConflictStatus::Pending,
        };

        assert_eq!(conflict.id, "test3");
        assert_eq!(conflict.status, ConflictStatus::Pending);
    }
}
