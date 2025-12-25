//! Intent types and builder for the Orpheon Protocol.
//!
//! An Intent is the core primitive - a declaration of a desired future state.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::error::{OrpheonError, Result};
use crate::types::Priority;

/// An Intent is a declaration of a desired future state.
/// It is immutable once signed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    /// Unique identifier for this intent.
    pub id: Uuid,

    /// The semantic type of the intent (e.g., "provision_gpu_cluster", "book_flight").
    pub kind: String,

    /// Hard constraints that MUST be met.
    pub constraints: Vec<Constraint>,

    /// Optimization preferences (e.g., minimize cost vs minimize latency).
    pub preferences: Vec<Preference>,

    /// The budget allowed for this execution (time + money).
    pub budget: Budget,

    /// The temporal window in which this intent is valid.
    pub validity_window: TimeWindow,

    /// Priority level for this intent.
    pub priority: Priority,

    /// Arbitrary metadata for the intent.
    #[serde(default)]
    pub metadata: serde_json::Value,

    /// Cryptographic signature of the issuer.
    pub signature: Option<Signature>,

    /// Timestamp when the intent was created.
    pub created_at: DateTime<Utc>,

    /// Parent intent ID (for recursive intents).
    pub parent_id: Option<Uuid>,
}

/// Hard constraint that MUST be satisfied.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Constraint {
    /// State must match expression (e.g., "region == 'US-EAST'").
    StateMatch { expression: String },

    /// Resource usage must be within limit (e.g., "total_cost < 5.00").
    ResourceLimit { resource: String, limit: f64 },

    /// SLA metric must meet threshold (e.g., "latency < 200ms").
    Sla { metric: String, threshold: u64, unit: String },

    /// Must complete before a specific time.
    Deadline { by: DateTime<Utc> },

    /// Must use specific provider/node.
    Provider { node_id: String },

    /// Geographic restriction.
    GeoFence { regions: Vec<String>, allowed: bool },

    /// Custom constraint with arbitrary data.
    Custom { name: String, data: serde_json::Value },
}

/// Optimization preference (soft constraint).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preference {
    /// What to optimize (e.g., "cost", "latency", "reliability").
    pub objective: String,

    /// Direction: minimize or maximize.
    pub direction: OptimizationDirection,

    /// Weight for multi-objective optimization (0.0 to 1.0).
    pub weight: f32,
}

/// Optimization direction.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationDirection {
    Minimize,
    Maximize,
}

/// Budget for intent execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Budget {
    /// Maximum monetary cost allowed.
    pub max_cost: Option<f64>,

    /// Currency for the cost (e.g., "USD").
    pub currency: String,

    /// Maximum execution time in milliseconds.
    pub max_duration_ms: Option<u64>,

    /// Maximum number of retries allowed.
    pub max_retries: u32,
}

impl Budget {
    /// Create a new budget with USD cost limit.
    pub fn usd(amount: f64) -> Self {
        Self {
            max_cost: Some(amount),
            currency: "USD".to_string(),
            max_duration_ms: None,
            max_retries: 3,
        }
    }

    /// Set maximum duration.
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.max_duration_ms = Some(duration_ms);
        self
    }

    /// Set maximum retries.
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
}

/// Time window during which an intent is valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    /// Earliest time the intent can begin execution.
    pub not_before: Option<DateTime<Utc>>,

    /// Latest time by which the intent must complete.
    pub not_after: Option<DateTime<Utc>>,
}

impl Default for TimeWindow {
    fn default() -> Self {
        Self {
            not_before: None,
            not_after: Some(Utc::now() + Duration::hours(24)),
        }
    }
}

impl TimeWindow {
    /// Create a time window that is valid for the given duration from now.
    pub fn valid_for(duration: Duration) -> Self {
        Self {
            not_before: None,
            not_after: Some(Utc::now() + duration),
        }
    }

    /// Check if the current time is within the window.
    pub fn is_valid_now(&self) -> bool {
        let now = Utc::now();
        let after_start = self.not_before.map_or(true, |t| now >= t);
        let before_end = self.not_after.map_or(true, |t| now <= t);
        after_start && before_end
    }
}

/// Cryptographic signature for intent authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signature {
    /// The algorithm used (e.g., "ed25519", "secp256k1").
    pub algorithm: String,

    /// The public key of the signer (hex-encoded).
    pub public_key: String,

    /// The signature bytes (hex-encoded).
    pub signature: String,

    /// Timestamp of when the signature was created.
    pub signed_at: DateTime<Utc>,
}

/// Builder for creating Intents with a fluent API.
#[derive(Debug, Default)]
pub struct IntentBuilder {
    kind: Option<String>,
    constraints: Vec<Constraint>,
    preferences: Vec<Preference>,
    budget: Budget,
    validity_window: TimeWindow,
    priority: Priority,
    metadata: serde_json::Value,
    parent_id: Option<Uuid>,
}

impl IntentBuilder {
    /// Create a new IntentBuilder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the kind of intent.
    pub fn kind(mut self, kind: impl Into<String>) -> Self {
        self.kind = Some(kind.into());
        self
    }

    /// Add a constraint.
    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Add a state match constraint.
    pub fn state_match(self, expression: impl Into<String>) -> Self {
        self.constraint(Constraint::StateMatch {
            expression: expression.into(),
        })
    }

    /// Add a resource limit constraint.
    pub fn resource_limit(self, resource: impl Into<String>, limit: f64) -> Self {
        self.constraint(Constraint::ResourceLimit {
            resource: resource.into(),
            limit,
        })
    }

    /// Add an SLA constraint.
    pub fn sla(self, metric: impl Into<String>, threshold: u64, unit: impl Into<String>) -> Self {
        self.constraint(Constraint::Sla {
            metric: metric.into(),
            threshold,
            unit: unit.into(),
        })
    }

    /// Add a preference.
    pub fn preference(mut self, preference: Preference) -> Self {
        self.preferences.push(preference);
        self
    }

    /// Add a minimize objective.
    pub fn minimize(self, objective: impl Into<String>, weight: f32) -> Self {
        self.preference(Preference {
            objective: objective.into(),
            direction: OptimizationDirection::Minimize,
            weight,
        })
    }

    /// Add a maximize objective.
    pub fn maximize(self, objective: impl Into<String>, weight: f32) -> Self {
        self.preference(Preference {
            objective: objective.into(),
            direction: OptimizationDirection::Maximize,
            weight,
        })
    }

    /// Set the budget.
    pub fn budget(mut self, budget: Budget) -> Self {
        self.budget = budget;
        self
    }

    /// Set the validity window.
    pub fn validity_window(mut self, window: TimeWindow) -> Self {
        self.validity_window = window;
        self
    }

    /// Set the priority.
    pub fn priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Set metadata.
    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }

    /// Set parent intent ID (for recursive intents).
    pub fn parent(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    /// Build the Intent.
    pub fn build(self) -> Result<Intent> {
        let kind = self.kind.ok_or_else(|| OrpheonError::IntentInvalid {
            intent_id: None,
            message: "Intent kind is required".to_string(),
        })?;

        Ok(Intent {
            id: Uuid::new_v4(),
            kind,
            constraints: self.constraints,
            preferences: self.preferences,
            budget: self.budget,
            validity_window: self.validity_window,
            priority: self.priority,
            metadata: self.metadata,
            signature: None,
            created_at: Utc::now(),
            parent_id: self.parent_id,
        })
    }
}

impl Intent {
    /// Create a new IntentBuilder.
    pub fn builder() -> IntentBuilder {
        IntentBuilder::new()
    }

    /// Calculate a hash of the intent content (for signing).
    pub fn content_hash(&self) -> String {
        let content = serde_json::json!({
            "id": self.id,
            "kind": self.kind,
            "constraints": self.constraints,
            "preferences": self.preferences,
            "budget": self.budget,
            "validity_window": self.validity_window,
            "priority": self.priority,
            "metadata": self.metadata,
            "created_at": self.created_at,
            "parent_id": self.parent_id,
        });

        let mut hasher = Sha256::new();
        hasher.update(content.to_string().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Validate the intent.
    pub fn validate(&self) -> Result<()> {
        // Check kind is not empty
        if self.kind.trim().is_empty() {
            return Err(OrpheonError::IntentInvalid {
                intent_id: Some(self.id),
                message: "Intent kind cannot be empty".to_string(),
            });
        }

        // Check validity window
        if !self.validity_window.is_valid_now() {
            return Err(OrpheonError::IntentInvalid {
                intent_id: Some(self.id),
                message: "Intent is outside its validity window".to_string(),
            });
        }

        // Validate preference weights
        let total_weight: f32 = self.preferences.iter().map(|p| p.weight).sum();
        if !self.preferences.is_empty() && (total_weight - 1.0).abs() > 0.01 {
            return Err(OrpheonError::IntentInvalid {
                intent_id: Some(self.id),
                message: format!("Preference weights must sum to 1.0, got {}", total_weight),
            });
        }

        Ok(())
    }

    /// Check if this intent has a parent (is part of a recursive chain).
    pub fn is_child(&self) -> bool {
        self.parent_id.is_some()
    }
}

// Add hex dependency for content_hash
fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
}

// Use the local function instead of the hex crate
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        super::hex_encode(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_builder() {
        let intent = Intent::builder()
            .kind("provision_gpu_cluster")
            .resource_limit("count", 8.0)
            .sla("latency", 200, "ms")
            .minimize("cost", 0.7)
            .maximize("speed", 0.3)
            .budget(Budget::usd(100.0))
            .build()
            .unwrap();

        assert_eq!(intent.kind, "provision_gpu_cluster");
        assert_eq!(intent.constraints.len(), 2);
        assert_eq!(intent.preferences.len(), 2);
    }

    #[test]
    fn test_intent_validation() {
        let intent = Intent::builder().kind("test").build().unwrap();
        assert!(intent.validate().is_ok());
    }

    #[test]
    fn test_intent_builder_missing_kind() {
        let result = Intent::builder().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_builder() {
        let budget = Budget::usd(50.0).with_duration(5000).with_retries(5);
        assert_eq!(budget.max_cost, Some(50.0));
        assert_eq!(budget.max_duration_ms, Some(5000));
        assert_eq!(budget.max_retries, 5);
    }

    #[test]
    fn test_time_window_validity() {
        let window = TimeWindow::valid_for(Duration::hours(1));
        assert!(window.is_valid_now());
    }
}
