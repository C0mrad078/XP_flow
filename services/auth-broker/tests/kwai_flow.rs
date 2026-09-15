use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use xpflow_auth_broker::config::KwaiCredentials;
use xpflow_auth_broker::db;
use xpflow_auth_broker::providers::kwai::KwaiProviderClient;
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
    let credentials = KwaiCredentials {
        app_id: "test-app-id".to_string(),
        app_secret: "test-app-secret".to_string(),
    };
    let kwai = Arc::new(KwaiProviderClient::with_base_url(
        credentials,
        &mock_server.uri(),
    ));
    let state = Arc::new(AppState::from_parts(
        Store::new(pool),
        [11u8; 32],
        None,
        Some(kwai),
        "http://127.0.0.1:8787".to_string(),
    ));
    routes::router(state)
}

async fn post_json(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn get(app: &axum::Router, uri: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
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

#[tokio::test]
async fn start_returns_a_pending_session_and_an_authorize_url() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;

    let (status, body) = post_json(
        &app,
        "/v1/auth/kwai/start",
        json!({ "workspace_id": "workspace-1", "channel_id": "channel-1" }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let session_id = body["session_id"].as_str().unwrap();
    assert!(body["authorize_url"]
        .as_str()
        .unwrap()
        .contains("app_id=test-app-id"));

    let (status, status_body) = get(&app, &format!("/v1/auth/sessions/{session_id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(status_body["status"], "pending");
}

#[tokio::test]
async fn callback_with_the_correct_state_completes_the_session() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/oauth2/access_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": 1,
            "access_token": "kwai-access-token",
            "refresh_token": "kwai-refresh-token",
            "expires_in": 3600,
        })))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": 1,
            "open_id": "kwai-user-1",
            "nick_name": "Football Cuts",
            "head_url": "https://example.com/avatar.png",
        })))
        .mount(&mock_server)
        .await;

    let app = test_app(&mock_server).await;
    let (_, start_body) = post_json(
        &app,
        "/v1/auth/kwai/start",
        json!({ "workspace_id": "w1", "channel_id": "c1" }),
    )
    .await;
    let session_id = start_body["session_id"].as_str().unwrap().to_string();
    let authorize_url = start_body["authorize_url"].as_str().unwrap();
    let state_token = url::Url::parse(authorize_url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.to_string())
        .unwrap();

    let callback_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/v1/auth/kwai/callback?code=kwai-code&state={state_token}"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(callback_response.status(), StatusCode::OK);

    let (_, status_body) = get(&app, &format!("/v1/auth/sessions/{session_id}")).await;
    assert_eq!(status_body["status"], "completed");
    assert_eq!(
        status_body["connection"]["provider_account_id"],
        "kwai-user-1"
    );
}

#[tokio::test]
async fn callback_with_an_unknown_state_never_completes_any_session() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;
    let (_, start_body) = post_json(
        &app,
        "/v1/auth/kwai/start",
        json!({ "workspace_id": "w1", "channel_id": "c1" }),
    )
    .await;
    let session_id = start_body["session_id"].as_str().unwrap().to_string();

    let callback_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/v1/auth/kwai/callback?code=kwai-code&state=not-the-real-state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // The callback endpoint always renders a page for the browser (it
    // can't redirect the user to a JSON error) — the important assertion
    // is that the *session* was never completed by a forged state.
    assert_eq!(callback_response.status(), StatusCode::OK);

    let (_, status_body) = get(&app, &format!("/v1/auth/sessions/{session_id}")).await;
    assert_eq!(status_body["status"], "pending");
}

#[tokio::test]
async fn an_unknown_session_id_returns_not_found() {
    let mock_server = MockServer::start().await;
    let app = test_app(&mock_server).await;
    let (status, body) = get(&app, "/v1/auth/sessions/does-not-exist").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], "SESSION_NOT_FOUND");
}
