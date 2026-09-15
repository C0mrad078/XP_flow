use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::{ConnectOptions, Executor};
use std::str::FromStr;
use tower::limit::ConcurrencyLimitLayer;
use tower::ServiceExt;
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;

use xpflow_auth_broker::db;
use xpflow_auth_broker::routes;
use xpflow_auth_broker::state::AppState;
use xpflow_auth_broker::store::Store;

fn temp_db_path() -> String {
    let dir =
        std::env::temp_dir().join(format!("xpflow-auth-broker-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("broker.db").display().to_string()
}

async fn get(app: axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// A session's expiry is enforced even though nothing has actively
/// "expired" it — `GET /v1/auth/sessions/:id` must compute this on read
/// (section 46: an abandoned auth attempt must not stay valid forever).
#[tokio::test]
async fn an_expired_pending_session_reports_expired_on_status_check() {
    let db_path = temp_db_path();
    let pool = db::init_pool(&db_path).await.unwrap();

    // Seed a session directly with an already-past expires_at — this is
    // the same database file the app's own Store will read from.
    let mut conn = SqliteConnectOptions::from_str(&db_path)
        .unwrap()
        .connect()
        .await
        .unwrap();
    conn.execute(
        "INSERT INTO oauth_sessions (id, platform, workspace_id, channel_id, state, status, created_at, expires_at) \
         VALUES ('expired-session', 'kwai', 'w1', 'c1', 'state-1', 'pending', '2020-01-01T00:00:00Z', '2020-01-01T00:10:00Z')",
    )
    .await
    .unwrap();

    let state = Arc::new(AppState::from_parts(
        Store::new(pool),
        [3u8; 32],
        None,
        None,
        "http://127.0.0.1:8787".to_string(),
    ));
    let app = routes::router(state);

    let (status, body) = get(app, "/v1/auth/sessions/expired-session").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "expired");
}

#[tokio::test]
async fn refreshing_an_unknown_connection_returns_not_found_not_a_crash() {
    let pool = db::init_pool(&temp_db_path()).await.unwrap();
    let state = Arc::new(AppState::from_parts(
        Store::new(pool),
        [4u8; 32],
        None,
        None,
        "http://127.0.0.1:8787".to_string(),
    ));
    let app = routes::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/connections/does-not-exist/refresh")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

/// Exercises the exact middleware stack `main.rs` installs in production
/// (section 26: "request size limits") — a request larger than the cap is
/// rejected before it ever reaches a handler or touches the database.
#[tokio::test]
async fn an_oversized_request_body_is_rejected_before_reaching_a_handler() {
    let pool = db::init_pool(&temp_db_path()).await.unwrap();
    let state = Arc::new(AppState::from_parts(
        Store::new(pool),
        [5u8; 32],
        None,
        None,
        "http://127.0.0.1:8787".to_string(),
    ));

    let app = routes::router(state)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::GATEWAY_TIMEOUT,
            Duration::from_secs(5),
        ))
        .layer(RequestBodyLimitLayer::new(1024))
        .layer(ConcurrencyLimitLayer::new(16));

    let oversized_code = "a".repeat(10_000);
    let body = json!({
        "session_id": "s", "workspace_id": "w", "code": oversized_code, "code_verifier": "v", "redirect_uri": "r"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/tiktok/exchange")
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

/// Section 86: nothing in a broker error response ever echoes a request
/// field back — a client-supplied "code" value must never reappear in the
/// error body even when the request is otherwise rejected.
#[tokio::test]
async fn error_responses_never_echo_a_submitted_secret_looking_field() {
    let pool = db::init_pool(&temp_db_path()).await.unwrap();
    let state = Arc::new(AppState::from_parts(
        Store::new(pool),
        [6u8; 32],
        None,
        None,
        "http://127.0.0.1:8787".to_string(),
    ));
    let app = routes::router(state);

    let secret_marker = "sk_super_secret_marker_value";
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/auth/tiktok/exchange")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "session_id": "s", "workspace_id": "w", "code": secret_marker, "code_verifier": "v", "redirect_uri": "r"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        !text.contains(secret_marker),
        "response body must never echo a submitted code value"
    );
}
