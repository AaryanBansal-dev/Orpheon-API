//! Quick Start Example
//!
//! This example demonstrates the basic usage of the Orpheon Protocol.

use orpheon_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<(), OrpheonError> {
    // Connect to an Orpheon node
    let client = OrpheonClient::connect("http://localhost:3000").await?;

    // 1. Define what you want
    let intent = Intent::builder()
        .kind("provision_gpu_cluster")
        .resource_limit("count", 8.0)
        .sla("type", 100, "H100")  // Simplified for example
        .budget(Budget::usd(100.0))
        .minimize("cost", 0.6)
        .maximize("speed", 0.4)
        .build()?;

    // 2. Submit intent and subscribe to updates
    let mut plan_stream = client.submit(intent).await?;

    println!("🚀 Intent submitted, waiting for updates...");

    // 3. Process events
    while let Some(event) = plan_stream.next().await {
        match event {
            Event::Negotiating { estimated_cost, .. } => {
                println!("💬 Negotiating: estimated cost ${:.2}", estimated_cost);
            }
            Event::Executing { step_name, progress, .. } => {
                println!("⚙️  Executing: {} ({:.0}%)", step_name, progress * 100.0);
            }
            Event::Complete { artifact_id } => {
                println!("✅ Done! Artifact ID: {}", artifact_id);
                break;
            }
            Event::StatusUpdate { status, .. } => {
                println!("📊 Status: {}", status);
            }
            Event::Error { message } => {
                println!("❌ Error: {}", message);
                break;
            }
        }
    }

    Ok(())
}
