//! Orpheon client implementation.

use orpheon_core::{Budget, Intent, IntentBuilder, OrpheonError, Plan, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::stream::EventStream;

/// Client for interacting with an Orpheon node.
#[derive(Clone)]
pub struct OrpheonClient {
    /// Base URL of the Orpheon node.
    base_url: String,
    
    /// HTTP client.
    http_client: reqwest::Client,
}

/// Response from submitting an intent.
#[derive(Debug, Deserialize)]
struct SubmitResponse {
    id: Uuid,
    status: String,
    message: String,
}

/// Response with intent details.
#[derive(Debug, Deserialize)]
pub struct IntentResponse {
    pub id: Uuid,
    pub kind: String,
    pub status: String,
    pub plan_id: Option<Uuid>,
    pub artifact_id: Option<Uuid>,
    pub error: Option<String>,
    pub created_at: String,
}

/// Request body for submitting an intent.
#[derive(Debug, Serialize)]
struct SubmitRequest {
    kind: String,
    constraints: Vec<serde_json::Value>,
    preferences: Vec<serde_json::Value>,
    budget: Option<BudgetRequest>,
    metadata: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct BudgetRequest {
    max_cost: Option<f64>,
    currency: Option<String>,
    max_duration_ms: Option<u64>,
    max_retries: Option<u32>,
}

impl OrpheonClient {
    /// Connect to an Orpheon node.
    pub async fn connect(url: &str) -> Result<Self> {
        let base_url = url.trim_end_matches('/').to_string();
        let http_client = reqwest::Client::new();
        
        // Verify connection with health check
        let health_url = format!("{}/health", base_url);
        http_client
            .get(&health_url)
            .send()
            .await
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?
            .error_for_status()
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?;
        
        Ok(Self {
            base_url,
            http_client,
        })
    }
    
    /// Submit an intent and get a stream of events.
    pub async fn submit(&self, intent: Intent) -> Result<EventStream> {
        // Submit the intent via REST
        let url = format!("{}/api/v1/intent", self.base_url);
        
        let request = SubmitRequest {
            kind: intent.kind.clone(),
            constraints: intent.constraints.iter().map(|c| serde_json::to_value(c).unwrap()).collect(),
            preferences: intent.preferences.iter().map(|p| serde_json::to_value(p).unwrap()).collect(),
            budget: Some(BudgetRequest {
                max_cost: intent.budget.max_cost,
                currency: Some(intent.budget.currency.clone()),
                max_duration_ms: intent.budget.max_duration_ms,
                max_retries: Some(intent.budget.max_retries),
            }),
            metadata: intent.metadata.clone(),
        };
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OrpheonError::Internal(format!("Failed to submit intent: {}", error_text)));
        }
        
        let submit_response: SubmitResponse = response
            .json()
            .await
            .map_err(|e| OrpheonError::SerializationError(e.to_string()))?;
        
        // Create WebSocket stream for updates
        let ws_url = format!(
            "{}/ws/intent/{}",
            self.base_url.replace("http://", "ws://").replace("https://", "wss://"),
            submit_response.id
        );
        
        EventStream::connect(&ws_url, submit_response.id).await
    }
    
    /// Get the status of an intent.
    pub async fn get_intent(&self, id: Uuid) -> Result<IntentResponse> {
        let url = format!("{}/api/v1/intent/{}", self.base_url, id);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?;
        
        if response.status().as_u16() == 404 {
            return Err(OrpheonError::NotFound {
                resource_type: "Intent".to_string(),
                id: id.to_string(),
            });
        }
        
        response
            .json()
            .await
            .map_err(|e| OrpheonError::SerializationError(e.to_string()))
    }
    
    /// Get the plan for an intent.
    pub async fn get_plan(&self, intent_id: Uuid) -> Result<Plan> {
        let url = format!("{}/api/v1/intent/{}/plan", self.base_url, intent_id);
        
        let response = self.http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?;
        
        if response.status().as_u16() == 404 {
            return Err(OrpheonError::NotFound {
                resource_type: "Plan".to_string(),
                id: intent_id.to_string(),
            });
        }
        
        response
            .json()
            .await
            .map_err(|e| OrpheonError::SerializationError(e.to_string()))
    }
    
    /// Cancel an intent.
    pub async fn cancel(&self, id: Uuid) -> Result<()> {
        let url = format!("{}/api/v1/intent/{}", self.base_url, id);
        
        let response = self.http_client
            .delete(&url)
            .send()
            .await
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?;
        
        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(OrpheonError::Internal(format!("Failed to cancel intent: {}", error_text)));
        }
        
        Ok(())
    }
    
    /// Simulate an intent without executing.
    pub async fn simulate(&self, intent: Intent) -> Result<SimulationResult> {
        let url = format!("{}/api/v1/simulate", self.base_url);
        
        let request = serde_json::json!({
            "kind": intent.kind,
            "constraints": [],
            "preferences": [],
            "budget": {
                "max_cost": intent.budget.max_cost,
                "max_duration_ms": intent.budget.max_duration_ms,
            }
        });
        
        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| OrpheonError::ConnectionError(e.to_string()))?;
        
        response
            .json()
            .await
            .map_err(|e| OrpheonError::SerializationError(e.to_string()))
    }
}

/// Result of a simulation.
#[derive(Debug, Deserialize)]
pub struct SimulationResult {
    pub simulation_id: Uuid,
    pub success: bool,
    pub estimated_cost: f64,
    pub estimated_duration_ms: u64,
    pub confidence_score: f32,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}
