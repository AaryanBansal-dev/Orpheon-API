//! # Orpheon SDK
//!
//! Client SDK for interacting with Orpheon nodes.

pub mod client;
pub mod stream;

pub use client::OrpheonClient;
pub use stream::EventStream;

/// Prelude module for common imports.
pub mod prelude {
    pub use crate::client::OrpheonClient;
    pub use crate::stream::EventStream;
    pub use orpheon_core::prelude::*;
}
