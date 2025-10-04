use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware::from_fn_with_state,
};
use dotenvy::dotenv;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod app_state;
mod cache;
mod config;
mod db;
mod middleware;
mod models;
mod routes;
mod services;
mod telemetry;
mod utils;

use crate::app_state::AppState;
use crate::config::Settings;
use crate::middleware::auth::{admin_auth_middleware, security_auth_middleware};
use crate::middleware::trace_id::trace_id_middleware;
use crate::telemetry::{init_logging, init_metrics, log_shutdown, log_startup};
use routes::{auth_routes, health_routes, quotes_routes, security_routes};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // Load configuration first
    let settings =
        Settings::new().map_err(|e| anyhow::anyhow!("Failed to load configuration: {}", e))?;

    // Initialize structured logging with settings
    init_logging(&settings)?;

    // Initialize Prometheus metrics
    init_metrics()?;

    // Create application state with configuration
    let app_state = Arc::new(AppState::new().await?);

    // Configure CORS using settings
    let allowed_origins = settings
        .cors
        .allowed_origins
        .iter()
        .map(|origin| origin.trim().parse::<HeaderValue>().unwrap())
        .collect::<Vec<_>>();

    // Parse allowed methods from settings
    let allowed_methods: Vec<Method> = settings
        .cors
        .allowed_methods
        .iter()
        .map(|method| method.parse().unwrap_or(Method::GET))
        .collect();

    let cors = CorsLayer::new()
        .allow_methods(allowed_methods)
        .allow_origin(allowed_origins)
        .allow_headers(
            settings
                .cors
                .allowed_headers
                .iter()
                .map(|h| h.parse().unwrap())
                .collect::<Vec<_>>(),
        )
        .allow_credentials(settings.cors.allow_credentials);

    // Build the application router with selective middleware layers
    let app = Router::new()
        .merge(health_routes())
        .merge(quotes_routes())
        // Protected routes with specific authentication
        .merge(security_routes().route_layer(from_fn_with_state(
            app_state.clone(),
            security_auth_middleware,
        )))
        .merge(
            auth_routes().route_layer(from_fn_with_state(app_state.clone(), admin_auth_middleware)),
        )
        .with_state(app_state.clone())
        .layer(
            ServiceBuilder::new()
                // Trace ID middleware (outermost - runs first)
                .layer(axum::middleware::from_fn(trace_id_middleware))
                // Tracing middleware for request/response logging
                .layer(TraceLayer::new_for_http())
                // CORS middleware
                .layer(cors),
        );

    // Start the server using settings
    let server_address = settings.server_address();
    let listener = tokio::net::TcpListener::bind(&server_address).await?;

    // Log startup information
    log_startup(&settings.server.host, settings.server.port);
    tracing::info!("Health check available at http://{}/health", server_address);

    // Set up graceful shutdown
    let shutdown_signal = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        log_shutdown();
    };

    // Start the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    tracing::info!("Axum server has fully shut down and main is exiting.");

    Ok(())
}
