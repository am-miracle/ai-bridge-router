use axum::{
    Router,
    http::{HeaderValue, Method},
    middleware::from_fn_with_state,
};
use dotenvy::dotenv;
use std::{env, sync::Arc};
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
use crate::middleware::auth::{admin_auth_middleware, security_auth_middleware};
use crate::telemetry::{init_logging, init_metrics, log_shutdown, log_startup};
use routes::{auth_routes, health_routes, quotes_routes, security_routes};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // Initialize structured logging
    init_logging()?;

    // Initialize Prometheus metrics
    init_metrics()?;

    // Create application state with database and Redis connections
    let app_state = Arc::new(AppState::new().await?);

    // Get server configuration
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("PORT must be a valid u16");

    // Configure CORS
    let allowed_origins = env::var("ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://0.0.0.0:3000".to_string())
        .split(',')
        .map(|origin| origin.trim().parse::<HeaderValue>().unwrap())
        .collect::<Vec<_>>();

    let cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_origin(allowed_origins)
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
        ])
        .allow_credentials(true);

    // Build the application router with all middleware layers
    let app = Router::new()
        .merge(health_routes())
        .merge(quotes_routes())
        .merge(security_routes())
        .merge(auth_routes())
        .with_state(app_state.clone())
        .layer(
            ServiceBuilder::new()
                // Tracing middleware for request/response logging
                .layer(TraceLayer::new_for_http())
                // CORS middleware
                .layer(cors),
        )
        .route_layer(from_fn_with_state(
            app_state.clone(),
            security_auth_middleware,
        ))
        .route_layer(from_fn_with_state(app_state.clone(), admin_auth_middleware));

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;

    // Log startup information
    log_startup(&host, port);
    tracing::info!("Health check available at http://{}:{}/health", host, port);
    tracing::info!(
        "Metrics endpoint available at http://{}:{}/metrics",
        host,
        port
    );
    tracing::info!(
        "Security endpoints available at http://{}:{}/security/* (API key required)",
        host,
        port
    );
    tracing::info!(
        "Auth endpoints available at http://{}:{}/auth/* (admin API key required)",
        host,
        port
    );

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

    Ok(())
}
