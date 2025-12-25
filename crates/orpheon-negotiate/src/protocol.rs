//! Negotiation protocol messages.

use chrono::{DateTime, Utc};
use orpheon_core::Plan;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Message types for the negotiation protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NegotiationMessage {
    /// Server offers a plan to the client.
    Offer(Proposal),
    
    /// Client accepts the current proposal.
    Accept { proposal_id: Uuid },
    
    /// Client rejects the current proposal.
    Reject { proposal_id: Uuid, reason: String },
    
    /// Client counter-offers with modified constraints.
    Counter(CounterOffer),
    
    /// Server acknowledges acceptance and begins execution.
    Confirmed { proposal_id: Uuid, execution_id: Uuid },
    
    /// Negotiation failed.
    Failed { reason: String },
    
    /// Ping for keepalive.
    Ping { timestamp: DateTime<Utc> },
    
    /// Pong response for keepalive.
    Pong { timestamp: DateTime<Utc> },
}

/// A proposal from the server to the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proposal {
    /// Unique ID for this proposal.
    pub id: Uuid,
    
    /// The intent being negotiated.
    pub intent_id: Uuid,
    
    /// The proposed plan.
    pub plan: Plan,
    
    /// Quoted cost for execution.
    pub quoted_cost: f64,
    
    /// Currency for the cost.
    pub currency: String,
    
    /// Estimated latency in milliseconds.
    pub estimated_latency_ms: u64,
    
    /// Guaranteed SLA metrics.
    pub sla_guarantees: Vec<SlaGuarantee>,
    
    /// When this proposal expires.
    pub expires_at: DateTime<Utc>,
    
    /// Proposal version (for counter-offers).
    pub version: u32,
    
    /// Metadata.
    pub metadata: serde_json::Value,
}

/// An SLA guarantee offered in a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlaGuarantee {
    /// The metric being guaranteed (e.g., "uptime", "latency_p99").
    pub metric: String,
    
    /// The threshold value.
    pub threshold: f64,
    
    /// Unit of measurement.
    pub unit: String,
    
    /// Penalty if SLA is not met.
    pub penalty: Option<f64>,
}

/// A counter-offer from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterOffer {
    /// Reference to the proposal being countered.
    pub proposal_id: Uuid,
    
    /// Requested maximum cost.
    pub max_cost: Option<f64>,
    
    /// Requested maximum latency.
    pub max_latency_ms: Option<u64>,
    
    /// Additional constraints to apply.
    pub additional_constraints: Vec<String>,
    
    /// Preferences to adjust.
    pub preference_adjustments: Vec<PreferenceAdjustment>,
    
    /// Message to the server.
    pub message: Option<String>,
}

/// Adjustment to a preference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreferenceAdjustment {
    /// The preference to adjust.
    pub objective: String,
    
    /// New weight (0.0 to 1.0).
    pub weight: f32,
}

impl Proposal {
    /// Create a new proposal.
    pub fn new(intent_id: Uuid, plan: Plan) -> Self {
        Self {
            id: Uuid::new_v4(),
            intent_id,
            plan: plan.clone(),
            quoted_cost: plan.estimated_cost,
            currency: "USD".to_string(),
            estimated_latency_ms: plan.estimated_latency_ms,
            sla_guarantees: Vec::new(),
            expires_at: Utc::now() + chrono::Duration::minutes(5),
            version: 1,
            metadata: serde_json::Value::Null,
        }
    }
    
    /// Check if the proposal has expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
    
    /// Add an SLA guarantee.
    pub fn with_sla(mut self, metric: impl Into<String>, threshold: f64, unit: impl Into<String>) -> Self {
        self.sla_guarantees.push(SlaGuarantee {
            metric: metric.into(),
            threshold,
            unit: unit.into(),
            penalty: None,
        });
        self
    }
}

impl CounterOffer {
    /// Create a new counter-offer.
    pub fn new(proposal_id: Uuid) -> Self {
        Self {
            proposal_id,
            max_cost: None,
            max_latency_ms: None,
            additional_constraints: Vec::new(),
            preference_adjustments: Vec::new(),
            message: None,
        }
    }
    
    /// Request a lower cost.
    pub fn with_max_cost(mut self, cost: f64) -> Self {
        self.max_cost = Some(cost);
        self
    }
    
    /// Request lower latency.
    pub fn with_max_latency(mut self, latency_ms: u64) -> Self {
        self.max_latency_ms = Some(latency_ms);
        self
    }
    
    /// Add a message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orpheon_core::{Plan, PlanningStrategy};

    #[test]
    fn test_proposal_creation() {
        let intent_id = Uuid::new_v4();
        let plan = Plan::new(intent_id, PlanningStrategy::Heuristic);
        
        let proposal = Proposal::new(intent_id, plan)
            .with_sla("latency_p99", 200.0, "ms");
        
        assert_eq!(proposal.intent_id, intent_id);
        assert_eq!(proposal.sla_guarantees.len(), 1);
        assert!(!proposal.is_expired());
    }

    #[test]
    fn test_counter_offer() {
        let proposal_id = Uuid::new_v4();
        
        let counter = CounterOffer::new(proposal_id)
            .with_max_cost(50.0)
            .with_max_latency(1000)
            .with_message("Need it cheaper");
        
        assert_eq!(counter.max_cost, Some(50.0));
        assert!(counter.message.is_some());
    }

    #[test]
    fn test_message_serialization() {
        let msg = NegotiationMessage::Accept { proposal_id: Uuid::new_v4() };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("accept"));
        
        let deserialized: NegotiationMessage = serde_json::from_str(&json).unwrap();
        matches!(deserialized, NegotiationMessage::Accept { .. });
    }
}
