//! Core execution engine.

use std::sync::Arc;

use orpheon_core::{ExecutionArtifact, ExecutionEvent, IntentStatus, Outcome, Plan};
use orpheon_planner::planner::PlanningState;
use orpheon_planner::Planner;
use tokio::time::{sleep, Duration};
use tracing::{error, info};

use crate::state::AppState;

/// The core execution engine.
pub struct Engine {
    state: AppState,
}

impl Engine {
    /// Create a new engine.
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
    
    /// Run the engine's main loop.
    pub async fn run(self: Arc<Self>) {
        info!("🔧 Engine started");
        
        loop {
            // Process pending intents
            self.process_pending_intents().await;
            
            // Small delay to prevent busy-waiting
            sleep(Duration::from_millis(100)).await;
        }
    }
    
    /// Process intents that are in Received or Planning state.
    async fn process_pending_intents(&self) {
        // Collect IDs of intents to process (to avoid borrow issues)
        let pending_ids: Vec<uuid::Uuid> = {
            let intents = self.state.intents.read().await;
            intents
                .iter()
                .filter(|(_, record)| record.status == IntentStatus::Received)
                .map(|(id, _)| *id)
                .collect()
        };

        // Process one intent at a time
        for id in pending_ids {
            self.start_planning(id).await;
            break; // Process one at a time
        }
    }
    
    /// Start planning for an intent.
    async fn start_planning(&self, intent_id: uuid::Uuid) {
        info!("📋 Starting planning for intent {}", intent_id);
        
        // Update status to Planning
        self.state.update_intent_status(intent_id, IntentStatus::Planning).await;
        
        // Get the intent
        let record = match self.state.get_intent(intent_id).await {
            Some(r) => r,
            None => {
                error!("Intent {} not found", intent_id);
                return;
            }
        };
        
        // Generate a plan
        let initial_state = PlanningState::default();
        let plan_result = self.state.planner.plan(&record.intent, &initial_state).await;
        
        match plan_result {
            Ok(plan) => {
                info!("✅ Plan generated for intent {} with {} steps", intent_id, plan.steps.len());
                
                // Store the plan
                self.state.store_plan(plan.clone()).await;
                
                // For simplicity, skip negotiation and go straight to execution
                self.state.update_intent_status(intent_id, IntentStatus::Executing).await;
                
                // Execute the plan
                self.execute_plan(intent_id, plan).await;
            }
            Err(e) => {
                error!("❌ Planning failed for intent {}: {}", intent_id, e);
                
                // Update status to Failed
                let mut intents = self.state.intents.write().await;
                if let Some(record) = intents.get_mut(&intent_id) {
                    record.status = IntentStatus::Failed;
                    record.error = Some(e.to_string());
                }
            }
        }
    }
    
    /// Execute a plan.
    async fn execute_plan(&self, intent_id: uuid::Uuid, plan: Plan) {
        info!("🚀 Executing plan for intent {}", intent_id);
        
        let record = match self.state.get_intent(intent_id).await {
            Some(r) => r,
            None => {
                error!("Intent {} not found during execution", intent_id);
                return;
            }
        };
        
        // Create artifact
        let mut artifact = ExecutionArtifact::new(
            record.intent.clone(),
            plan.clone(),
            Outcome::Success,
        );
        
        // Execute each step (simplified simulation)
        for step in &plan.steps {
            info!("  📌 Executing step: {}", step.name);
            
            // Record start event
            artifact.add_event(ExecutionEvent::step_started(step.id));
            
            // Simulate execution time
            let duration_ms = step.estimated_duration_ms.max(50);
            sleep(Duration::from_millis(duration_ms)).await;
            
            // Record completion event
            artifact.add_event(ExecutionEvent::step_completed(step.id, duration_ms));
            
            artifact.actual_cost += step.estimated_cost;
        }
        
        info!("✅ Execution complete for intent {}", intent_id);
        
        // Store the artifact
        self.state.store_artifact(artifact).await;
    }
}
