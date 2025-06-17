//! Basic API usage example for Matrixon API
//!
//! Demonstrates how to:
//! - Create an Axum router with Matrixon API routes
//! - Add middleware
//! - Start the server

use matrixon_api::routes::create_router;
use matrixon_api::server::Server;
use tracing_subscriber;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create router with all routes and middleware
    let router = create_router();

    // Create and start server
    let mut server = Server::new_default();
    server.start().await.unwrap();
}
