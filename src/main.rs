use axum::{
    http::{HeaderValue, Method},
    Router,
};
use dotenvy::dotenv;
use std::env;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod db;
mod cache;
mod routes;
mod services;
mod models;
mod utils;
mod telemetry;

use routes::health::health_routes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bridge_router=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    // Build the application router
    let app = Router::new()
        .merge(health_routes())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors),
        );

    // Start the server
    let listener = tokio::net::TcpListener::bind(format!("{}:{}", host, port)).await?;

    tracing::info!("Bridge Router server starting on {}:{}", host, port);
    tracing::info!("Health check available at http://{}:{}/health", host, port);

    axum::serve(listener, app).await?;

    Ok(())
}
