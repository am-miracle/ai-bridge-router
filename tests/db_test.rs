use bridge_router::config::Settings;
use bridge_router::db::pool::{
    _get_pool_stats as get_pool_stats, check_connection, init_pg_pool_with_config,
};
use bridge_router::utils::errors::AppResult;

/// Test PostgreSQL connectivity
#[tokio::test]
async fn test_postgresql_connectivity() -> AppResult<()> {
    // This test requires a running PostgreSQL instance
    // Skip if DATABASE_URL is not properly configured
    match std::env::var("DATABASE_URL") {
        Ok(url) if !url.is_empty() && url != "postgres://localhost/bridge_router" => {
            let settings =
                Settings::new().map_err(|e| anyhow::anyhow!("Failed to load settings: {}", e))?;
            let pool = init_pg_pool_with_config(&settings).await?;

            // Test basic connectivity
            assert!(check_connection(&pool).await);

            // Test pool statistics
            let stats = get_pool_stats(&pool).await?;
            assert!(stats.total_connections > 0);

            println!("PostgreSQL connectivity test passed");
            println!("Pool stats: {:?}", stats);

            Ok(())
        }
        _ => {
            println!("Skipping PostgreSQL test - DATABASE_URL not configured for testing");
            Ok(())
        }
    }
}
