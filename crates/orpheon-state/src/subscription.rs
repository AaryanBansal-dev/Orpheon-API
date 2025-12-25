//! State subscription system.

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::store::StateEntry;

/// A state change event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateChangeEvent {
    /// The key that changed.
    pub key: String,
    
    /// The new value (None if deleted).
    pub new_value: Option<StateEntry>,
    
    /// The previous value (None if new key).
    pub old_value: Option<StateEntry>,
    
    /// Type of change.
    pub change_type: ChangeType,
    
    /// Timestamp of the change.
    pub timestamp: DateTime<Utc>,
}

/// Type of state change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// New key created.
    Created,
    /// Existing key updated.
    Updated,
    /// Key deleted.
    Deleted,
}

/// Filter for subscriptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionFilter {
    /// Key prefix to match.
    pub key_prefix: Option<String>,
    
    /// Specific keys to watch.
    pub keys: Option<Vec<String>>,
    
    /// Change types to watch.
    pub change_types: Option<Vec<ChangeType>>,
    
    /// SEL (State Expression Language) expression (simplified).
    pub expression: Option<String>,
}

impl Default for SubscriptionFilter {
    fn default() -> Self {
        Self {
            key_prefix: None,
            keys: None,
            change_types: None,
            expression: None,
        }
    }
}

impl SubscriptionFilter {
    /// Create a filter for a key prefix.
    pub fn prefix(prefix: impl Into<String>) -> Self {
        Self {
            key_prefix: Some(prefix.into()),
            ..Default::default()
        }
    }
    
    /// Create a filter for specific keys.
    pub fn keys(keys: Vec<String>) -> Self {
        Self {
            keys: Some(keys),
            ..Default::default()
        }
    }
    
    /// Check if an event matches this filter.
    pub fn matches(&self, event: &StateChangeEvent) -> bool {
        // Check key prefix
        if let Some(ref prefix) = self.key_prefix {
            if !event.key.starts_with(prefix) {
                return false;
            }
        }
        
        // Check specific keys
        if let Some(ref keys) = self.keys {
            if !keys.contains(&event.key) {
                return false;
            }
        }
        
        // Check change types
        if let Some(ref types) = self.change_types {
            if !types.contains(&event.change_type) {
                return false;
            }
        }
        
        // TODO: Implement SEL expression matching
        
        true
    }
}

/// A subscription to state changes.
pub struct StateSubscription {
    /// Unique ID for this subscription.
    pub id: Uuid,
    
    /// Filter for this subscription.
    pub filter: SubscriptionFilter,
    
    /// Receiver for events.
    pub receiver: broadcast::Receiver<StateChangeEvent>,
}

/// Manager for state subscriptions.
pub struct SubscriptionManager {
    /// Sender for broadcasting events.
    sender: broadcast::Sender<StateChangeEvent>,
    
    /// Active subscriptions.
    subscriptions: Arc<RwLock<HashMap<Uuid, SubscriptionFilter>>>,
}

impl SubscriptionManager {
    /// Create a new subscription manager.
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(1000);
        Self {
            sender,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Subscribe to state changes with a filter.
    pub async fn subscribe(&self, filter: SubscriptionFilter) -> StateSubscription {
        let id = Uuid::new_v4();
        let receiver = self.sender.subscribe();
        
        let mut subs = self.subscriptions.write().await;
        subs.insert(id, filter.clone());
        
        StateSubscription { id, filter, receiver }
    }
    
    /// Unsubscribe from state changes.
    pub async fn unsubscribe(&self, id: Uuid) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(&id);
    }
    
    /// Publish a state change event.
    pub async fn publish(&self, event: StateChangeEvent) {
        // Broadcast to all subscribers (they filter locally)
        let _ = self.sender.send(event);
    }
    
    /// Get the number of active subscriptions.
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }
}

impl Default for SubscriptionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_prefix() {
        let filter = SubscriptionFilter::prefix("intent:");
        
        let event = StateChangeEvent {
            key: "intent:123".to_string(),
            new_value: None,
            old_value: None,
            change_type: ChangeType::Created,
            timestamp: Utc::now(),
        };
        
        assert!(filter.matches(&event));
        
        let non_matching = StateChangeEvent {
            key: "plan:456".to_string(),
            new_value: None,
            old_value: None,
            change_type: ChangeType::Created,
            timestamp: Utc::now(),
        };
        
        assert!(!filter.matches(&non_matching));
    }

    #[test]
    fn test_filter_keys() {
        let filter = SubscriptionFilter::keys(vec!["key1".to_string(), "key2".to_string()]);
        
        let event = StateChangeEvent {
            key: "key1".to_string(),
            new_value: None,
            old_value: None,
            change_type: ChangeType::Updated,
            timestamp: Utc::now(),
        };
        
        assert!(filter.matches(&event));
    }

    #[tokio::test]
    async fn test_subscription_manager() {
        let manager = SubscriptionManager::new();
        
        let sub = manager.subscribe(SubscriptionFilter::default()).await;
        
        assert_eq!(manager.subscription_count().await, 1);
        
        manager.unsubscribe(sub.id).await;
        
        assert_eq!(manager.subscription_count().await, 0);
    }
}
