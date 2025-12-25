//! Negotiation session management.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use orpheon_core::{Intent, OrpheonError, Plan, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

use crate::protocol::{CounterOffer, NegotiationMessage, Proposal};

/// State of a negotiation session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NegotiationState {
    /// Waiting for initial proposal.
    Pending,
    /// Proposal sent, waiting for client response.
    ProposalSent,
    /// Client countered, re-planning.
    Countered,
    /// Client accepted the proposal.
    Accepted,
    /// Negotiation rejected/failed.
    Rejected,
    /// Negotiation timed out.
    TimedOut,
    /// Execution has begun.
    Executing,
}

/// A negotiation session between client and server.
pub struct NegotiationSession {
    /// Unique ID for this session.
    pub id: Uuid,
    
    /// The intent being negotiated.
    pub intent: Intent,
    
    /// Current state of the negotiation.
    state: Arc<RwLock<NegotiationState>>,
    
    /// Current proposal (if any).
    current_proposal: Arc<RwLock<Option<Proposal>>>,
    
    /// History of proposals.
    proposal_history: Arc<RwLock<Vec<Proposal>>>,
    
    /// History of counter-offers.
    counter_history: Arc<RwLock<Vec<CounterOffer>>>,
    
    /// When the session started.
    pub started_at: DateTime<Utc>,
    
    /// When the session times out.
    pub timeout_at: DateTime<Utc>,
    
    /// Maximum number of negotiation rounds.
    pub max_rounds: u32,
    
    /// Current round number.
    round: Arc<RwLock<u32>>,
    
    /// Channel for outgoing messages.
    outgoing_tx: mpsc::Sender<NegotiationMessage>,
    
    /// Channel for incoming messages.
    incoming_rx: Arc<RwLock<mpsc::Receiver<NegotiationMessage>>>,
}

impl NegotiationSession {
    /// Create a new negotiation session.
    pub fn new(intent: Intent, timeout_seconds: u64, max_rounds: u32) -> (Self, mpsc::Sender<NegotiationMessage>, mpsc::Receiver<NegotiationMessage>) {
        let (outgoing_tx, outgoing_rx) = mpsc::channel(100);
        let (incoming_tx, incoming_rx) = mpsc::channel(100);
        
        let session = Self {
            id: Uuid::new_v4(),
            intent,
            state: Arc::new(RwLock::new(NegotiationState::Pending)),
            current_proposal: Arc::new(RwLock::new(None)),
            proposal_history: Arc::new(RwLock::new(Vec::new())),
            counter_history: Arc::new(RwLock::new(Vec::new())),
            started_at: Utc::now(),
            timeout_at: Utc::now() + chrono::Duration::seconds(timeout_seconds as i64),
            max_rounds,
            round: Arc::new(RwLock::new(0)),
            outgoing_tx,
            incoming_rx: Arc::new(RwLock::new(incoming_rx)),
        };
        
        (session, incoming_tx, outgoing_rx)
    }
    
    /// Get the current state.
    pub async fn state(&self) -> NegotiationState {
        *self.state.read().await
    }
    
    /// Get the current proposal.
    pub async fn current_proposal(&self) -> Option<Proposal> {
        self.current_proposal.read().await.clone()
    }
    
    /// Get the current round number.
    pub async fn current_round(&self) -> u32 {
        *self.round.read().await
    }
    
    /// Check if the session has timed out.
    pub fn is_timed_out(&self) -> bool {
        Utc::now() > self.timeout_at
    }
    
    /// Send a proposal to the client.
    pub async fn send_proposal(&self, plan: Plan) -> Result<Proposal> {
        let mut state = self.state.write().await;
        let mut round = self.round.write().await;
        
        if *round >= self.max_rounds {
            return Err(OrpheonError::NegotiationRejected {
                intent_id: self.intent.id,
                reason: "Maximum negotiation rounds exceeded".to_string(),
            });
        }
        
        *round += 1;
        
        let proposal = Proposal::new(self.intent.id, plan);
        
        // Store proposal
        {
            let mut current = self.current_proposal.write().await;
            *current = Some(proposal.clone());
        }
        
        {
            let mut history = self.proposal_history.write().await;
            history.push(proposal.clone());
        }
        
        // Send message
        self.outgoing_tx
            .send(NegotiationMessage::Offer(proposal.clone()))
            .await
            .map_err(|_| OrpheonError::Internal("Failed to send proposal".to_string()))?;
        
        *state = NegotiationState::ProposalSent;
        
        Ok(proposal)
    }
    
    /// Process an acceptance from the client.
    pub async fn accept(&self, proposal_id: Uuid) -> Result<Uuid> {
        let mut state = self.state.write().await;
        
        let current = self.current_proposal.read().await;
        let proposal = current.as_ref().ok_or_else(|| {
            OrpheonError::NegotiationRejected {
                intent_id: self.intent.id,
                reason: "No active proposal to accept".to_string(),
            }
        })?;
        
        if proposal.id != proposal_id {
            return Err(OrpheonError::NegotiationRejected {
                intent_id: self.intent.id,
                reason: "Proposal ID mismatch".to_string(),
            });
        }
        
        if proposal.is_expired() {
            return Err(OrpheonError::NegotiationRejected {
                intent_id: self.intent.id,
                reason: "Proposal has expired".to_string(),
            });
        }
        
        *state = NegotiationState::Accepted;
        
        let execution_id = Uuid::new_v4();
        
        self.outgoing_tx
            .send(NegotiationMessage::Confirmed {
                proposal_id,
                execution_id,
            })
            .await
            .map_err(|_| OrpheonError::Internal("Failed to send confirmation".to_string()))?;
        
        Ok(execution_id)
    }
    
    /// Process a counter-offer from the client.
    pub async fn counter(&self, counter: CounterOffer) -> Result<()> {
        let mut state = self.state.write().await;
        
        let current = self.current_proposal.read().await;
        let proposal = current.as_ref().ok_or_else(|| {
            OrpheonError::NegotiationRejected {
                intent_id: self.intent.id,
                reason: "No active proposal to counter".to_string(),
            }
        })?;
        
        if proposal.id != counter.proposal_id {
            return Err(OrpheonError::NegotiationRejected {
                intent_id: self.intent.id,
                reason: "Counter-offer references wrong proposal".to_string(),
            });
        }
        
        // Store counter-offer
        {
            let mut history = self.counter_history.write().await;
            history.push(counter);
        }
        
        *state = NegotiationState::Countered;
        
        Ok(())
    }
    
    /// Reject the negotiation.
    pub async fn reject(&self, reason: String) -> Result<()> {
        let mut state = self.state.write().await;
        *state = NegotiationState::Rejected;
        
        self.outgoing_tx
            .send(NegotiationMessage::Failed { reason })
            .await
            .map_err(|_| OrpheonError::Internal("Failed to send rejection".to_string()))?;
        
        Ok(())
    }
    
    /// Get the last counter-offer.
    pub async fn last_counter(&self) -> Option<CounterOffer> {
        let history = self.counter_history.read().await;
        history.last().cloned()
    }
    
    /// Get proposal history.
    pub async fn proposal_history(&self) -> Vec<Proposal> {
        self.proposal_history.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orpheon_core::{Intent, Plan, PlanningStrategy};

    fn create_test_intent() -> Intent {
        Intent::builder()
            .kind("test")
            .build()
            .unwrap()
    }

    #[tokio::test]
    async fn test_session_creation() {
        let intent = create_test_intent();
        let (session, _incoming_tx, _outgoing_rx) = NegotiationSession::new(intent.clone(), 60, 5);
        
        assert_eq!(session.intent.id, intent.id);
        assert_eq!(session.state().await, NegotiationState::Pending);
        assert_eq!(session.current_round().await, 0);
    }

    #[tokio::test]
    async fn test_send_proposal() {
        let intent = create_test_intent();
        let (session, _incoming_tx, mut outgoing_rx) = NegotiationSession::new(intent.clone(), 60, 5);
        
        let plan = Plan::new(intent.id, PlanningStrategy::Deterministic);
        let proposal = session.send_proposal(plan).await.unwrap();
        
        assert_eq!(session.state().await, NegotiationState::ProposalSent);
        assert_eq!(session.current_round().await, 1);
        
        // Check message was sent
        let msg = outgoing_rx.recv().await.unwrap();
        matches!(msg, NegotiationMessage::Offer(_));
    }

    #[tokio::test]
    async fn test_max_rounds() {
        let intent = create_test_intent();
        let (session, _incoming_tx, _outgoing_rx) = NegotiationSession::new(intent.clone(), 60, 2);
        
        let plan = Plan::new(intent.id, PlanningStrategy::Deterministic);
        
        // First two rounds should succeed
        session.send_proposal(plan.clone()).await.unwrap();
        session.send_proposal(plan.clone()).await.unwrap();
        
        // Third round should fail
        let result = session.send_proposal(plan).await;
        assert!(result.is_err());
    }
}
