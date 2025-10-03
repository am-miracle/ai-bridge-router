pub mod auth;
pub mod bridges;
pub mod health;
pub mod quotes;
pub mod security;

pub use auth::auth_routes;
pub use health::health_routes;
pub use quotes::quotes_routes;
pub use security::security_routes;
// pub use bridges::*;
