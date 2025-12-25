//! # Orpheon Negotiate
//!
//! WebSocket-based negotiation protocol for the Orpheon Protocol.

pub mod protocol;
pub mod session;

pub use protocol::{NegotiationMessage, Proposal, CounterOffer};
pub use session::{NegotiationSession, NegotiationState};
