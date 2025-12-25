//! Application state.

use std::collections::HashMap;
use std::sync::Arc;

use orpheon_core::{ExecutionArtifact, Intent, Plan};
use orpheon_planner::AStarPlanner;
use orpheon_state::InMemoryStateStore;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    /// Active intents.
    pub intents: Arc<RwLock<HashMap<Uuid, IntentRecord>>>,
    
    /// Generated plans.
    pub plans: Arc<RwLock<HashMap<Uuid, Plan>>>,
    
    /// Execution artifacts.
    pub artifacts: Arc<RwLock<HashMap<Uuid, ExecutionArtifact>>>,
    
    /// The planner engine.
    pub planner: Arc<AStarPlanner>,
    
    /// The state store.
    pub state_store: Arc<InMemoryStateStore>,
}

/// Record of an intent with its status.
#[derive(Clone)]
pub struct IntentRecord {
    /// The intent.
    pub intent: Intent,
    
    /// Current status.
    pub status: orpheon_core::IntentStatus,
    
    /// Associated plan ID (if generated).
    pub plan_id: Option<Uuid>,
    
    /// Associated artifact ID (if complete).
    pub artifact_id: Option<Uuid>,
    
    /// Error message (if failed).
    pub error: Option<String>,
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self {
            intents: Arc::new(RwLock::new(HashMap::new())),
            plans: Arc::new(RwLock::new(HashMap::new())),
            artifacts: Arc::new(RwLock::new(HashMap::new())),
            planner: Arc::new(AStarPlanner::new()),
            state_store: Arc::new(InMemoryStateStore::new()),
        }
    }
    
    /// Store an intent.
    pub async fn store_intent(&self, intent: Intent) {
        let record = IntentRecord {
            intent: intent.clone(),
            status: orpheon_core::IntentStatus::Received,
            plan_id: None,
            artifact_id: None,
            error: None,
        };
        
        let mut intents = self.intents.write().await;
        intents.insert(intent.id, record);
    }
    
    /// Get an intent by ID.
    pub async fn get_intent(&self, id: Uuid) -> Option<IntentRecord> {
        let intents = self.intents.read().await;
        intents.get(&id).cloned()
    }
    
    /// Update intent status.
    pub async fn update_intent_status(&self, id: Uuid, status: orpheon_core::IntentStatus) {
        let mut intents = self.intents.write().await;
        if let Some(record) = intents.get_mut(&id) {
            record.status = status;
        }
    }
    
    /// Store a plan.
    pub async fn store_plan(&self, plan: Plan) {
        let intent_id = plan.intent_id;
        let plan_id = plan.id;
        
        let mut plans = self.plans.write().await;
        plans.insert(plan_id, plan);
        
        // Update intent record
        let mut intents = self.intents.write().await;
        if let Some(record) = intents.get_mut(&intent_id) {
            record.plan_id = Some(plan_id);
        }
    }
    
    /// Get a plan by ID.
    pub async fn get_plan(&self, id: Uuid) -> Option<Plan> {
        let plans = self.plans.read().await;
        plans.get(&id).cloned()
    }
    
    /// Get plan by intent ID.
    pub async fn get_plan_for_intent(&self, intent_id: Uuid) -> Option<Plan> {
        let intents = self.intents.read().await;
        let plan_id = intents.get(&intent_id)?.plan_id?;
        drop(intents);
        
        self.get_plan(plan_id).await
    }
    
    /// Store an artifact.
    pub async fn store_artifact(&self, artifact: ExecutionArtifact) {
        let intent_id = artifact.intent.id;
        let artifact_id = artifact.id;
        
        let mut artifacts = self.artifacts.write().await;
        artifacts.insert(artifact_id, artifact);
        
        // Update intent record
        let mut intents = self.intents.write().await;
        if let Some(record) = intents.get_mut(&intent_id) {
            record.artifact_id = Some(artifact_id);
            record.status = orpheon_core::IntentStatus::Complete;
        }
    }
    
    /// Get an artifact by ID.
    pub async fn get_artifact(&self, id: Uuid) -> Option<ExecutionArtifact> {
        let artifacts = self.artifacts.read().await;
        artifacts.get(&id).cloned()
    }
    
    /// Get artifact by intent ID.
    pub async fn get_artifact_for_intent(&self, intent_id: Uuid) -> Option<ExecutionArtifact> {
        let intents = self.intents.read().await;
        let artifact_id = intents.get(&intent_id)?.artifact_id?;
        drop(intents);
        
        self.get_artifact(artifact_id).await
    }
    
    /// List all intents.
    pub async fn list_intents(&self) -> Vec<IntentRecord> {
        let intents = self.intents.read().await;
        intents.values().cloned().collect()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
