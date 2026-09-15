use std::sync::Arc;
use std::time::Duration;

use tower::limit::ConcurrencyLimitLayer;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use xpflow_auth_broker::config::Config;
use xpflow_auth_broker::state::AppState;
use xpflow_auth_broker::store::Store;
use xpflow_auth_broker::{db, routes};

/// Section 26: hard caps so a malformed/abusive request can't tie up the
/// broker — this is a narrow, security-sensitive service, not a general
/// web app that needs elaborate rate limiting.
const MAX_REQUEST_BODY_BYTES: usize = 16 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_CONCURRENT_REQUESTS: usize = 64;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap()),
        )
        .init();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            // Section 85: fail fast and clearly rather than starting in a
            // half-usable state with an invalid/missing master key.
            eprintln!("xpflow-auth-broker: configuration error: {err}");
            eprintln!("See .env.example for required environment variables.");
            std::process::exit(1);
        }
    };

    if config.tiktok.is_none() {
        tracing::warn!("TIKTOK_CLIENT_KEY/TIKTOK_CLIENT_SECRET not set — TikTok requests will report BROKER_CONFIGURATION_ERROR");
    }
    if config.kwai.is_none() {
        tracing::warn!("KWAI_APP_ID/KWAI_APP_SECRET not set — Kwai requests will report BROKER_CONFIGURATION_ERROR");
    }

    let bind_addr = config.bind_addr.clone();
    let pool = db::init_pool(&config.database_url)
        .await
        .expect("failed to initialize the broker's database");
    tracing::info!("auth broker database migrations applied");

    let state = Arc::new(AppState::new(config, Store::new(pool)));

    let app = routes::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::with_status_code(
            axum::http::StatusCode::GATEWAY_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(RequestBodyLimitLayer::new(MAX_REQUEST_BODY_BYTES))
        .layer(ConcurrencyLimitLayer::new(MAX_CONCURRENT_REQUESTS))
        // Section 26: no browser ever calls this API directly (only the
        // desktop app's own HTTP client and the providers' server-to-
        // server redirects do), so CORS stays fully locked down rather
        // than opened for a use case that doesn't exist.
        .layer(CorsLayer::new());

    tracing::info!(%bind_addr, "xpflow-auth-broker listening");
    let listener = tokio::net::TcpListener::bind(&bind_addr)
        .await
        .expect("failed to bind broker listener");
    axum::serve(listener, app)
        .await
        .expect("broker server error");
}
