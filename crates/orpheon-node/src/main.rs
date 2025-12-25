//! # Orpheon Node
//!
//! Main Orpheon node binary with API server.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{
    routing::{get, post, delete},
    Router,
};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

mod api;
mod engine;
mod state;

use engine::Engine;
use state::AppState;

/// Run the Orpheon node server.
pub async fn run_server(addr: SocketAddr) -> anyhow::Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Orpheon Node starting...");

    // Create shared application state
    let state = AppState::new();

    // Create the engine
    let engine = Arc::new(Engine::new(state.clone()));

    // Start the engine background task
    let engine_clone = engine.clone();
    tokio::spawn(async move {
        engine_clone.run().await;
    });

    // Build the router
    let app = create_router(state);

    info!("🌐 Listening on http://{}", addr);

    // Start the server
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create the API router.
fn create_router(state: AppState) -> Router {
    // CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        // Health check
        .route("/health", get(api::health::health_check))
        
        // Intent API
        .route("/api/v1/intent", post(api::intent::submit_intent))
        .route("/api/v1/intent/:id", get(api::intent::get_intent))
        .route("/api/v1/intent/:id", delete(api::intent::cancel_intent))
        .route("/api/v1/intent/:id/plan", get(api::intent::get_plan))
        .route("/api/v1/intent/:id/artifact", get(api::intent::get_artifact))
        .route("/api/v1/intents", get(api::intent::list_intents))
        
        // WebSocket endpoints
        .route("/ws/intent/:id", get(api::ws::intent_stream))
        .route("/ws/negotiate/:id", get(api::ws::negotiate_stream))
        .route("/ws/state", get(api::ws::state_stream))
        
        // Simulation endpoint
        .route("/api/v1/simulate", post(api::simulate::simulate_intent))
        
        // Add middleware
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    run_server(addr).await
}
