use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use xpflow_auth_broker::config::TikTokCredentials;
use xpflow_auth_broker::db;
use xpflow_auth_broker::providers::tiktok::TikTokProviderClient;
use xpflow_auth_broker::routes;
use xpflow_auth_broker::state::AppState;
use xpflow_auth_broker::store::Store;

fn temp_db_path() -> String {
    let dir =
        std::env::temp_dir().join(format!("xpflow-auth-broker-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("broker.db").display().to_string()
}

async fn test_app(mock_server: &MockServer) -> axum::Router {
    let pool = db::init_pool(&temp_db_path())
        .await
        .expect("test db should init");
    let credentials = TikTokCredentials {
        client_key: "test-client-key".to_string(),
        client_secret: "test-client-secret".to_string(),
    };
    let tiktok = Arc::new(TikTokProviderClient::with_base_url(
        credentials,
        &mock_server.uri(),
    ));
    let state = Arc::new(AppState::from_parts(
        Store::new(pool),
        [9u8; 32],
        Some(tiktok),
        None,
        "http://127.0.0.1:8787".to_string(),
    ));
    routes::router(state)
}

async fn mount_successful_token_and_identity(mock_server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "fake-access-token",
            "refresh_token": "fake-refresh-token",
            "expires_in": 3600,
            "refresh_expires_in": 86400,
            "scope": "user.info.basic"
        })))
        .mount(mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/user/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": { "user": { "open_id": "tiktok-user-1", "display_name": "Football Cuts", "avatar_url": null } },
            "error": { "code": "ok" }
        })))
        .mount(mock_server)
        .await;
}

async fn post_json(app: axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn successful_exchange_creates_a_connection() {
    let mock_server = MockServer::start().await;
    mount_successful_token_and_identity(&mock_server).await;
    let app = test_app(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/v1/auth/tiktok/exchange",
        json!({
            "session_id": "session-1",
            "workspace_id": "workspace-1",
            "code": "auth-code",
            "code_verifier": "verifier",
            "redirect_uri": "http://127.0.0.1:12345/oauth/tiktok/callback",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["provider_account_id"], "tiktok-user-1");
    assert_eq!(body["display_name"], "Football Cuts");
    assert!(body["connection_id"].is_string());
}

#[tokio::test]
async fn a_replayed_exchange_with_the_same_session_id_is_rejected() {
    let mock_server = MockServer::start().await;
    mount_successful_token_and_identity(&mock_server).await;
    let app = test_app(&mock_server).await;

    let request = json!({
        "session_id": "session-replay",
        "workspace_id": "workspace-1",
        "code": "auth-code",
        "code_verifier": "verifier",
        "redirect_uri": "http://127.0.0.1:12345/oauth/tiktok/callback",
    });

    let (first_status, _) =
        post_json(app.clone(), "/v1/auth/tiktok/exchange", request.clone()).await;
    assert_eq!(first_status, StatusCode::OK);

    let (second_status, second_body) = post_json(app, "/v1/auth/tiktok/exchange", request).await;
    assert_eq!(second_status, StatusCode::BAD_REQUEST);
    assert_eq!(second_body["code"], "AUTH_CODE_INVALID");
}

#[tokio::test]
async fn missing_required_fields_is_a_bad_request() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/v1/auth/tiktok/exchange",
        json!({ "session_id": "", "workspace_id": "", "code": "", "code_verifier": "", "redirect_uri": "" }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "BAD_REQUEST");
}

#[tokio::test]
async fn an_unconfigured_provider_reports_a_clean_configuration_error() {
    let pool = db::init_pool(&temp_db_path()).await.unwrap();
    let state = Arc::new(AppState::from_parts(
        Store::new(pool),
        [9u8; 32],
        None, // TikTok not configured
        None,
        "http://127.0.0.1:8787".to_string(),
    ));
    let app = routes::router(state);

    let (status, body) = post_json(
        app,
        "/v1/auth/tiktok/exchange",
        json!({
            "session_id": "s",
            "workspace_id": "w",
            "code": "c",
            "code_verifier": "v",
            "redirect_uri": "http://127.0.0.1:1/callback",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["code"], "BROKER_CONFIGURATION_ERROR");
}

#[tokio::test]
async fn an_invalid_grant_from_the_provider_maps_to_a_clean_code_invalid_error() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "error": "invalid_grant",
            "error_description": "the authorization code has expired"
        })))
        .mount(&mock_server)
        .await;
    let app = test_app(&mock_server).await;

    let (status, body) = post_json(
        app,
        "/v1/auth/tiktok/exchange",
        json!({
            "session_id": "session-invalid-grant",
            "workspace_id": "workspace-1",
            "code": "stale-code",
            "code_verifier": "verifier",
            "redirect_uri": "http://127.0.0.1:12345/oauth/tiktok/callback",
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], "AUTH_CODE_INVALID");
}

#[tokio::test]
async fn refresh_and_revoke_round_trip_through_a_real_connection() {
    let mock_server = MockServer::start().await;
    mount_successful_token_and_identity(&mock_server).await;
    let app = test_app(&mock_server).await;

    let (_, connect_body) = post_json(
        app.clone(),
        "/v1/auth/tiktok/exchange",
        json!({
            "session_id": "session-refresh",
            "workspace_id": "workspace-1",
            "code": "auth-code",
            "code_verifier": "verifier",
            "redirect_uri": "http://127.0.0.1:12345/oauth/tiktok/callback",
        }),
    )
    .await;
    let connection_id = connect_body["connection_id"].as_str().unwrap().to_string();

    Mock::given(method("POST"))
        .and(path("/oauth/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "rotated-access-token",
            "expires_in": 3600,
        })))
        .mount(&mock_server)
        .await;

    let refresh_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/connections/{connection_id}/refresh"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_response.status(), StatusCode::OK);

    Mock::given(method("POST"))
        .and(path("/oauth/revoke"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let revoke_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/connections/{connection_id}/revoke"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(revoke_response.status(), StatusCode::OK);
}
