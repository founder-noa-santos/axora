//! Architecture Decision Record (ADR) system.
//!
//! This module provides a system for tracking architectural decisions
//! with linking between related decisions and consequence tracking.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// ADR errors
#[derive(Error, Debug)]
pub enum AdrError {
    /// ADR not found
    #[error("ADR not found: {0}")]
    NotFound(String),

    /// Duplicate ADR ID
    #[error("duplicate ADR ID: {0}")]
    Duplicate(String),

    /// Invalid ADR status transition
    #[error("invalid status transition from {from:?} to {to:?}")]
    InvalidStatusTransition { from: AdrStatus, to: AdrStatus },

    /// Circular reference detected
    #[error("circular reference detected: {0}")]
    CircularReference(String),
}

/// Result type for ADR operations
pub type Result<T> = std::result::Result<T, AdrError>;

/// Status of an Architecture Decision Record
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AdrStatus {
    /// Decision is being proposed
    #[default]
    Proposed,
    /// Decision has been accepted
    Accepted,
    /// Decision is deprecated (replaced by newer decision)
    Deprecated,
    /// Decision has been superseded by another
    Superseded,
}

impl AdrStatus {
    /// Check if this status is active (not deprecated/superseded)
    pub fn is_active(&self) -> bool {
        matches!(self, AdrStatus::Proposed | AdrStatus::Accepted)
    }

    /// Check if transition to another status is valid
    pub fn can_transition_to(&self, to: &AdrStatus) -> bool {
        matches!(
            (self, to),
            (AdrStatus::Proposed, AdrStatus::Accepted)
                | (AdrStatus::Proposed, AdrStatus::Deprecated)
                | (AdrStatus::Accepted, AdrStatus::Deprecated)
                | (AdrStatus::Accepted, AdrStatus::Superseded)
        )
    }
}

/// Architecture Decision Record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Adr {
    /// Unique identifier (e.g., "AUTH-001", "CACHE-002")
    pub id: String,
    /// Title of the decision
    pub title: String,
    /// Current status
    pub status: AdrStatus,
    /// Context describing the problem
    pub context: String,
    /// The decision made
    pub decision: String,
    /// Consequences of this decision
    pub consequences: Vec<String>,
    /// Related ADR IDs
    pub related: Vec<String>,
    /// Unix timestamp when created
    pub created_at: u64,
    /// Unix timestamp when last updated
    pub updated_at: u64,
    /// Author of the ADR
    pub author: String,
}

impl Adr {
    /// Create a new ADR
    pub fn new(id: &str, title: &str, context: &str, decision: &str, author: &str) -> Self {
        let now = Utc::now().timestamp() as u64;
        Self {
            id: id.to_string(),
            title: title.to_string(),
            status: AdrStatus::Proposed,
            context: context.to_string(),
            decision: decision.to_string(),
            consequences: Vec::new(),
            related: Vec::new(),
            created_at: now,
            updated_at: now,
            author: author.to_string(),
        }
    }

    /// Accept this ADR
    pub fn accept(&mut self) -> Result<()> {
        if !self.status.can_transition_to(&AdrStatus::Accepted) {
            return Err(AdrError::InvalidStatusTransition {
                from: self.status.clone(),
                to: AdrStatus::Accepted,
            });
        }
        self.status = AdrStatus::Accepted;
        self.updated_at = Utc::now().timestamp() as u64;
        Ok(())
    }

    /// Deprecate this ADR
    pub fn deprecate(&mut self) -> Result<()> {
        if !self.status.can_transition_to(&AdrStatus::Deprecated) {
            return Err(AdrError::InvalidStatusTransition {
                from: self.status.clone(),
                to: AdrStatus::Deprecated,
            });
        }
        self.status = AdrStatus::Deprecated;
        self.updated_at = Utc::now().timestamp() as u64;
        Ok(())
    }

    /// Supersede this ADR
    pub fn supersede(&mut self) -> Result<()> {
        if !self.status.can_transition_to(&AdrStatus::Superseded) {
            return Err(AdrError::InvalidStatusTransition {
                from: self.status.clone(),
                to: AdrStatus::Superseded,
            });
        }
        self.status = AdrStatus::Superseded;
        self.updated_at = Utc::now().timestamp() as u64;
        Ok(())
    }

    /// Add a consequence
    pub fn add_consequence(&mut self, consequence: &str) {
        self.consequences.push(consequence.to_string());
        self.updated_at = Utc::now().timestamp() as u64;
    }

    /// Link to a related ADR
    pub fn add_related(&mut self, adr_id: &str) {
        if !self.related.contains(&adr_id.to_string()) {
            self.related.push(adr_id.to_string());
            self.updated_at = Utc::now().timestamp() as u64;
        }
    }

    /// Remove a related ADR link
    pub fn remove_related(&mut self, adr_id: &str) {
        self.related.retain(|id| id != adr_id);
        self.updated_at = Utc::now().timestamp() as u64;
    }

    /// Get age in days
    pub fn age_days(&self) -> u64 {
        let now = Utc::now().timestamp() as u64;
        (now - self.created_at) / 86400
    }
}

/// Log of Architecture Decision Records
pub struct AdrLog {
    /// Map of ADR ID to ADR
    adrs: HashMap<String, Adr>,
    /// Map of category prefix to ADR IDs (e.g., "AUTH" -> ["AUTH-001", "AUTH-002"])
    categories: HashMap<String, Vec<String>>,
}

impl AdrLog {
    /// Create a new empty ADR log
    pub fn new() -> Self {
        Self {
            adrs: HashMap::new(),
            categories: HashMap::new(),
        }
    }

    /// Add an ADR to the log
    pub fn add(&mut self, adr: Adr) -> Result<()> {
        if self.adrs.contains_key(&adr.id) {
            return Err(AdrError::Duplicate(adr.id.clone()));
        }

        // Extract category from ID (e.g., "AUTH" from "AUTH-001")
        if let Some(category) = extract_category(&adr.id) {
            self.categories
                .entry(category)
                .or_default()
                .push(adr.id.clone());
        }

        self.adrs.insert(adr.id.clone(), adr);

        Ok(())
    }

    /// Get an ADR by ID
    pub fn get(&self, adr_id: &str) -> Option<&Adr> {
        self.adrs.get(adr_id)
    }

    /// Get a mutable reference to an ADR by ID
    pub fn get_mut(&mut self, adr_id: &str) -> Option<&mut Adr> {
        self.adrs.get_mut(adr_id)
    }

    /// Link two ADRs together
    pub fn link(&mut self, adr_id: &str, related_id: &str) -> Result<()> {
        // Check both ADRs exist
        if !self.adrs.contains_key(adr_id) {
            return Err(AdrError::NotFound(adr_id.to_string()));
        }
        if !self.adrs.contains_key(related_id) {
            return Err(AdrError::NotFound(related_id.to_string()));
        }

        // Check for circular references
        if self.would_create_cycle(adr_id, related_id) {
            return Err(AdrError::CircularReference(format!(
                "Linking {} to {} would create a cycle",
                adr_id, related_id
            )));
        }

        // Add bidirectional links
        if let Some(adr) = self.adrs.get_mut(adr_id) {
            adr.add_related(related_id);
        }
        if let Some(related) = self.adrs.get_mut(related_id) {
            related.add_related(adr_id);
        }

        Ok(())
    }

    /// Remove a link between two ADRs
    pub fn unlink(&mut self, adr_id: &str, related_id: &str) {
        if let Some(adr) = self.adrs.get_mut(adr_id) {
            adr.remove_related(related_id);
        }
        if let Some(related) = self.adrs.get_mut(related_id) {
            related.remove_related(adr_id);
        }
    }

    /// Get all ADRs
    pub fn all(&self) -> Vec<&Adr> {
        self.adrs.values().collect()
    }

    /// Get ADRs by status
    pub fn by_status(&self, status: &AdrStatus) -> Vec<&Adr> {
        self.adrs
            .values()
            .filter(|adr| &adr.status == status)
            .collect()
    }

    /// Get ADRs by category
    pub fn by_category(&self, category: &str) -> Vec<&Adr> {
        self.categories
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| self.adrs.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get active ADRs (not deprecated/superseded)
    pub fn active(&self) -> Vec<&Adr> {
        self.adrs
            .values()
            .filter(|adr| adr.status.is_active())
            .collect()
    }

    /// Get count of ADRs
    pub fn len(&self) -> usize {
        self.adrs.len()
    }

    /// Check if log is empty
    pub fn is_empty(&self) -> bool {
        self.adrs.is_empty()
    }

    /// Get all categories
    pub fn categories(&self) -> Vec<&str> {
        self.categories.keys().map(|s| s.as_str()).collect()
    }

    /// Find ADRs by keyword search
    pub fn search(&self, keywords: &[&str]) -> Vec<&Adr> {
        let mut results: Vec<&Adr> = Vec::new();

        for adr in self.adrs.values() {
            let content = format!(
                "{} {} {} {}",
                adr.title,
                adr.context,
                adr.decision,
                adr.consequences.join(" ")
            )
            .to_lowercase();

            if keywords
                .iter()
                .any(|kw| content.contains(&kw.to_lowercase()))
            {
                results.push(adr);
            }
        }

        results
    }

    // === Internal Implementation ===

    /// Check if linking would create a cycle
    fn would_create_cycle(&self, from: &str, to: &str) -> bool {
        // Simple BFS to detect path from 'to' back to 'from'
        let mut visited: Vec<&str> = Vec::new();
        let mut queue: Vec<&str> = vec![to];

        while let Some(current) = queue.pop() {
            if current == from {
                return true;
            }

            if visited.contains(&current) {
                continue;
            }
            visited.push(current);

            if let Some(adr) = self.adrs.get(current) {
                for related in &adr.related {
                    queue.push(related.as_str());
                }
            }
        }

        false
    }
}

impl Default for AdrLog {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract category from ADR ID (e.g., "AUTH" from "AUTH-001")
fn extract_category(id: &str) -> Option<String> {
    id.split('-').next().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adr_creation() {
        let adr = Adr::new(
            "AUTH-001",
            "Use JWT for session management",
            "We need stateless authentication",
            "Use JWT tokens with 1 hour expiry",
            "agent-a",
        );

        assert_eq!(adr.id, "AUTH-001");
        assert_eq!(adr.title, "Use JWT for session management");
        assert_eq!(adr.status, AdrStatus::Proposed);
        assert_eq!(adr.author, "agent-a");
        assert!(adr.consequences.is_empty());
    }

    #[test]
    fn test_adr_status_transitions() {
        let mut adr = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");

        // Proposed -> Accepted (valid)
        assert!(adr.accept().is_ok());
        assert_eq!(adr.status, AdrStatus::Accepted);

        // Accepted -> Deprecated (valid)
        assert!(adr.deprecate().is_ok());
        assert_eq!(adr.status, AdrStatus::Deprecated);

        // Deprecated -> Accepted (invalid)
        assert!(adr.accept().is_err());
    }

    #[test]
    fn test_adr_proposed_to_deprecated() {
        let mut adr = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");

        // Proposed -> Deprecated (valid)
        assert!(adr.deprecate().is_ok());
        assert_eq!(adr.status, AdrStatus::Deprecated);
    }

    #[test]
    fn test_adr_add_consequence() {
        let mut adr = Adr::new(
            "CACHE-001",
            "Use Redis",
            "Need caching",
            "Use Redis",
            "agent-b",
        );

        adr.add_consequence("Faster response times");
        adr.add_consequence("Need to manage Redis instance");

        assert_eq!(adr.consequences.len(), 2);
        assert!(adr
            .consequences
            .contains(&"Faster response times".to_string()));
    }

    #[test]
    fn test_adr_add_related() {
        let mut adr = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");

        adr.add_related("AUTH-002");
        adr.add_related("SEC-001");

        assert_eq!(adr.related.len(), 2);
        assert!(adr.related.contains(&"AUTH-002".to_string()));
    }

    #[test]
    fn test_adr_linking() {
        let mut log = AdrLog::new();

        let adr1 = Adr::new("AUTH-001", "JWT Auth", "Context", "Decision", "author");
        let adr2 = Adr::new("AUTH-002", "Token Refresh", "Context", "Decision", "author");

        log.add(adr1).expect("Failed to add ADR 1");
        log.add(adr2).expect("Failed to add ADR 2");

        // Link ADRs
        log.link("AUTH-001", "AUTH-002").expect("Failed to link");

        // Verify bidirectional link
        let adr1 = log.get("AUTH-001").unwrap();
        let adr2 = log.get("AUTH-002").unwrap();

        assert!(adr1.related.contains(&"AUTH-002".to_string()));
        assert!(adr2.related.contains(&"AUTH-001".to_string()));
    }

    #[test]
    fn test_adr_unlink() {
        let mut log = AdrLog::new();

        let adr1 = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");
        let adr2 = Adr::new("AUTH-002", "Title", "Context", "Decision", "author");

        log.add(adr1).expect("Failed");
        log.add(adr2).expect("Failed");
        log.link("AUTH-001", "AUTH-002").expect("Failed to link");

        // Unlink
        log.unlink("AUTH-001", "AUTH-002");

        let adr1 = log.get("AUTH-001").unwrap();
        assert!(!adr1.related.contains(&"AUTH-002".to_string()));
    }

    #[test]
    fn test_adr_log_by_status() {
        let mut log = AdrLog::new();

        let mut adr1 = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");
        adr1.accept().expect("Failed to accept");

        let adr2 = Adr::new("AUTH-002", "Title", "Context", "Decision", "author");
        let adr3 = Adr::new("CACHE-001", "Title", "Context", "Decision", "author");

        log.add(adr1).expect("Failed");
        log.add(adr2).expect("Failed");
        log.add(adr3).expect("Failed");

        let accepted = log.by_status(&AdrStatus::Accepted);
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].id, "AUTH-001");

        let proposed = log.by_status(&AdrStatus::Proposed);
        assert_eq!(proposed.len(), 2);
    }

    #[test]
    fn test_adr_log_by_category() {
        let mut log = AdrLog::new();

        log.add(Adr::new(
            "AUTH-001", "Title", "Context", "Decision", "author",
        ))
        .expect("Failed");
        log.add(Adr::new(
            "AUTH-002", "Title", "Context", "Decision", "author",
        ))
        .expect("Failed");
        log.add(Adr::new(
            "CACHE-001",
            "Title",
            "Context",
            "Decision",
            "author",
        ))
        .expect("Failed");

        let auth_adrs = log.by_category("AUTH");
        assert_eq!(auth_adrs.len(), 2);

        let cache_adrs = log.by_category("CACHE");
        assert_eq!(cache_adrs.len(), 1);
    }

    #[test]
    fn test_adr_log_active() {
        let mut log = AdrLog::new();

        let mut adr1 = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");
        adr1.accept().expect("Failed");

        let mut adr2 = Adr::new("AUTH-002", "Title", "Context", "Decision", "author");
        adr2.deprecate().expect("Failed");

        let adr3 = Adr::new("CACHE-001", "Title", "Context", "Decision", "author");

        log.add(adr1).expect("Failed");
        log.add(adr2).expect("Failed");
        log.add(adr3).expect("Failed");

        let active = log.active();
        assert_eq!(active.len(), 2); // AUTH-001 (accepted) and CACHE-001 (proposed)
    }

    #[test]
    fn test_adr_search() {
        let mut log = AdrLog::new();

        log.add(Adr::new(
            "AUTH-001",
            "JWT Authentication",
            "Need stateless auth",
            "Use JWT tokens",
            "author",
        ))
        .expect("Failed");

        log.add(Adr::new(
            "CACHE-001",
            "Redis Caching",
            "Need caching layer",
            "Use Redis for sessions",
            "author",
        ))
        .expect("Failed");

        let results = log.search(&["jwt", "authentication"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "AUTH-001");

        let results2 = log.search(&["redis"]);
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].id, "CACHE-001");
    }

    #[test]
    fn test_adr_circular_reference_prevention() {
        let mut log = AdrLog::new();

        log.add(Adr::new("A-001", "Title", "Context", "Decision", "author"))
            .expect("Failed");
        log.add(Adr::new("A-002", "Title", "Context", "Decision", "author"))
            .expect("Failed");
        log.add(Adr::new("A-003", "Title", "Context", "Decision", "author"))
            .expect("Failed");

        // Create chain: A-001 -> A-002 -> A-003
        log.link("A-001", "A-002").expect("Failed");
        log.link("A-002", "A-003").expect("Failed");

        // Try to create cycle: A-003 -> A-001 (should fail)
        let result = log.link("A-003", "A-001");
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            AdrError::CircularReference(_)
        ));
    }

    #[test]
    fn test_adr_duplicate_prevention() {
        let mut log = AdrLog::new();

        let adr = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");
        log.add(adr).expect("Failed");

        let adr2 = Adr::new("AUTH-001", "Title 2", "Context 2", "Decision 2", "author");
        let result = log.add(adr2);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AdrError::Duplicate(_)));
    }

    #[test]
    fn test_adr_not_found() {
        let mut log = AdrLog::new();

        assert!(log.get("NONEXISTENT").is_none());

        let result = log.link("A-001", "A-002");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AdrError::NotFound(_)));
    }

    #[test]
    fn test_adr_age() {
        let adr = Adr::new("AUTH-001", "Title", "Context", "Decision", "author");

        // Just created, age should be 0
        assert_eq!(adr.age_days(), 0);
    }

    #[test]
    fn test_adr_categories() {
        let mut log = AdrLog::new();

        log.add(Adr::new(
            "AUTH-001", "Title", "Context", "Decision", "author",
        ))
        .expect("Failed");
        log.add(Adr::new(
            "CACHE-001",
            "Title",
            "Context",
            "Decision",
            "author",
        ))
        .expect("Failed");
        log.add(Adr::new(
            "API-001", "Title", "Context", "Decision", "author",
        ))
        .expect("Failed");

        let categories = log.categories();
        assert_eq!(categories.len(), 3);
        assert!(categories.contains(&"AUTH"));
        assert!(categories.contains(&"CACHE"));
        assert!(categories.contains(&"API"));
    }

    #[test]
    fn test_full_adr_workflow() {
        let mut log = AdrLog::new();

        // Step 1: Create ADR for authentication
        let auth_adr = Adr::new(
            "AUTH-001",
            "Use JWT for session management",
            "We need stateless authentication for our microservices architecture. \
             Current session-based auth doesn't scale well across services.",
            "Implement JWT-based authentication with:\n\
             - Access tokens (15 min expiry)\n\
             - Refresh tokens (7 day expiry)\n\
             - RS256 signing algorithm",
            "agent-a",
        );

        log.add(auth_adr).expect("Failed to add ADR");

        // Step 2: Accept the ADR
        log.get_mut("AUTH-001")
            .unwrap()
            .accept()
            .expect("Failed to accept");

        // Step 3: Create related ADR for token storage
        let storage_adr = Adr::new(
            "AUTH-002",
            "Store tokens in HttpOnly cookies",
            "Need secure client-side token storage",
            "Use HttpOnly, Secure, SameSite cookies for token storage",
            "agent-a",
        );

        log.add(storage_adr).expect("Failed to add ADR");

        // Step 4: Link related ADRs
        log.link("AUTH-001", "AUTH-002").expect("Failed to link");

        // Step 5: Add consequences
        log.get_mut("AUTH-001")
            .unwrap()
            .add_consequence("Increased token size in requests");

        // Step 6: Verify workflow
        let auth_adr = log.get("AUTH-001").unwrap();
        assert_eq!(auth_adr.status, AdrStatus::Accepted);
        assert_eq!(auth_adr.related.len(), 1);
        assert_eq!(auth_adr.consequences.len(), 1);

        let storage_adr = log.get("AUTH-002").unwrap();
        assert_eq!(storage_adr.status, AdrStatus::Proposed);
        assert!(storage_adr.related.contains(&"AUTH-001".to_string()));

        // Step 7: Search
        let results = log.search(&["jwt", "authentication"]);
        assert!(!results.is_empty());
    }
}
