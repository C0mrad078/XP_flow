use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain_capability::default_scopes_for;
use crate::error::BrokerError;
use crate::state::AppState;
use crate::store::SessionStatus;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/auth/tiktok/exchange", post(tiktok_exchange))
        .route("/v1/auth/kwai/start", post(kwai_start))
        .route("/v1/auth/kwai/callback", get(kwai_callback))
        .route("/v1/auth/sessions/:id", get(get_session_status))
        .route("/v1/connections/:id/refresh", post(refresh_connection))
        .route("/v1/connections/:id/revoke", post(revoke_connection))
        .route("/v1/connections/:id/status", get(connection_status))
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

// --- Shared response shapes (mirrors the desktop's broker_client.rs) ---

#[derive(Serialize)]
struct ConnectionView {
    connection_id: String,
    provider_account_id: String,
    display_name: Option<String>,
    username_or_handle: Option<String>,
    avatar_url: Option<String>,
    granted_scopes: Vec<String>,
    access_expires_at: Option<DateTime<Utc>>,
    refresh_expires_at: Option<DateTime<Utc>>,
}

impl From<&crate::store::Connection> for ConnectionView {
    fn from(c: &crate::store::Connection) -> Self {
        Self {
            connection_id: c.id.clone(),
            provider_account_id: c.provider_account_id.clone(),
            display_name: c.display_name.clone(),
            username_or_handle: c.username_or_handle.clone(),
            avatar_url: c.avatar_url.clone(),
            granted_scopes: c.granted_scopes.clone(),
            access_expires_at: c.access_expires_at,
            refresh_expires_at: c.refresh_expires_at,
        }
    }
}

// --- POST /v1/auth/tiktok/exchange -------------------------------------

#[derive(Deserialize)]
struct TikTokExchangeRequest {
    session_id: String,
    workspace_id: String,
    code: String,
    code_verifier: String,
    redirect_uri: String,
}

async fn tiktok_exchange(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TikTokExchangeRequest>,
) -> Result<Json<ConnectionView>, BrokerError> {
    if req.session_id.trim().is_empty()
        || req.code.trim().is_empty()
        || req.workspace_id.trim().is_empty()
    {
        return Err(BrokerError::BadRequest(
            "missing required fields".to_string(),
        ));
    }

    // Section 86: every request that carries a code/verifier is logged
    // through the redaction utility, never by hand — so a future call
    // site that adds a debug log here can't accidentally reintroduce a
    // secret leak by forgetting to redact one specific field.
    let mut loggable = serde_json::json!({
        "session_id": req.session_id,
        "workspace_id": req.workspace_id,
        "code": req.code,
        "code_verifier": req.code_verifier,
        "redirect_uri": req.redirect_uri,
    });
    crate::redact::redact_json(&mut loggable);
    tracing::debug!(request = %loggable, "tiktok exchange request received");

    let client = state
        .tiktok
        .clone()
        .ok_or(BrokerError::ProviderNotConfigured)?;

    let tokens = client
        .exchange_code(&req.code, &req.code_verifier, &req.redirect_uri)
        .await?;
    let identity = client.fetch_identity(&tokens.access_token).await?;

    // Section 59: reconnecting with a different real account must never
    // silently replace an existing identity's tokens under someone
    // else's row — `upsert_connection` only ever matches on
    // (workspace_id, platform, provider_account_id), so a different
    // account naturally becomes a *different* connection row; the
    // desktop layer is responsible for surfacing the "you authorized a
    // different account" prompt before calling this for a *reconnect* of
    // an existing `PlatformAccount`.
    //
    // Sections 22/82 (privacy minimization): beyond the bare workspace
    // id needed to scope identity-uniqueness, the broker learns nothing
    // about the desktop's channel, video titles, or anything else.
    let access_encrypted = state.cipher.encrypt(&tokens.access_token)?;
    let refresh_encrypted = tokens
        .refresh_token
        .as_deref()
        .map(|t| state.cipher.encrypt(t))
        .transpose()?;

    let connection = state
        .store
        .upsert_connection(
            "tiktok",
            &req.workspace_id,
            &identity.provider_account_id,
            identity.display_name.as_deref(),
            identity.username_or_handle.as_deref(),
            identity.avatar_url.as_deref(),
            &tokens.granted_scopes,
            &access_encrypted,
            refresh_encrypted.as_deref(),
            tokens.access_expires_at,
            tokens.refresh_expires_at,
        )
        .await?;

    state
        .store
        .create_completed_session(&req.session_id, "tiktok", &connection.id)
        .await?;

    Ok(Json(ConnectionView::from(&connection)))
}

// --- POST /v1/auth/kwai/start -------------------------------------------

#[derive(Deserialize)]
struct KwaiStartRequest {
    workspace_id: String,
    channel_id: String,
    // Kwai's flow is entirely broker-owned (section 17/18): the redirect
    // URI is always this broker's own registered callback, never a
    // caller-supplied one (section 25 — no arbitrary URL from a caller).
    // The field is intentionally not accepted here.
}

#[derive(Serialize)]
struct SessionStartView {
    session_id: String,
    authorize_url: String,
}

async fn kwai_start(
    State(state): State<Arc<AppState>>,
    Json(req): Json<KwaiStartRequest>,
) -> Result<Json<SessionStartView>, BrokerError> {
    if req.workspace_id.trim().is_empty() || req.channel_id.trim().is_empty() {
        return Err(BrokerError::BadRequest(
            "workspace_id and channel_id are required".to_string(),
        ));
    }
    let client = state
        .kwai
        .clone()
        .ok_or(BrokerError::ProviderNotConfigured)?;

    let redirect_uri = format!("{}/v1/auth/kwai/callback", state.public_base_url);
    let state_token = crate::domain_capability::generate_state();
    let session = state
        .store
        .create_session(
            "kwai",
            &req.workspace_id,
            &req.channel_id,
            &state_token,
            None,
            Some(&redirect_uri),
        )
        .await?;

    let authorize_url =
        client.build_authorize_url(&redirect_uri, &state_token, &default_scopes_for("kwai"));

    Ok(Json(SessionStartView {
        session_id: session.id,
        authorize_url,
    }))
}

// --- GET /v1/auth/kwai/callback ------------------------------------------

#[derive(Deserialize)]
struct KwaiCallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

async fn kwai_callback(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<KwaiCallbackParams>,
) -> axum::response::Html<&'static str> {
    let result = handle_kwai_callback(&state, params).await;
    if let Err(err) = result {
        tracing::warn!(error = %err, "kwai callback failed");
    }
    axum::response::Html(
        "<html><body style=\"font-family:sans-serif;text-align:center;padding-top:4rem\">\
         <h2>You can close this window</h2><p>Return to XP FLOW to finish connecting.</p></body></html>",
    )
}

async fn handle_kwai_callback(
    state: &AppState,
    params: KwaiCallbackParams,
) -> Result<(), BrokerError> {
    let Some(state_token) = params.state else {
        return Err(BrokerError::StateMismatch);
    };
    let session = state
        .store
        .find_session_by_state(&state_token)
        .await?
        .ok_or(BrokerError::StateMismatch)?;

    if let Some(error) = params.error {
        state
            .store
            .mark_session_failed(&session.id, "AUTH_CANCELLED", &error)
            .await?;
        return Ok(());
    }
    let Some(code) = params.code else {
        state
            .store
            .mark_session_failed(&session.id, "AUTH_CODE_INVALID", "missing code")
            .await?;
        return Ok(());
    };
    if session.is_expired(Utc::now()) {
        return Err(BrokerError::SessionExpired);
    }

    let client = state
        .kwai
        .clone()
        .ok_or(BrokerError::ProviderNotConfigured)?;
    let redirect_uri = session.redirect_uri.clone().unwrap_or_default();

    let outcome = async {
        let tokens = client.exchange_code(&code, &redirect_uri).await?;
        let identity = client.fetch_identity(&tokens.access_token).await?;
        let access_encrypted = state.cipher.encrypt(&tokens.access_token)?;
        let refresh_encrypted = tokens
            .refresh_token
            .as_deref()
            .map(|t| state.cipher.encrypt(t))
            .transpose()?;
        let connection = state
            .store
            .upsert_connection(
                "kwai",
                &session.workspace_id,
                &identity.provider_account_id,
                identity.display_name.as_deref(),
                identity.username_or_handle.as_deref(),
                identity.avatar_url.as_deref(),
                &tokens.granted_scopes,
                &access_encrypted,
                refresh_encrypted.as_deref(),
                tokens.access_expires_at,
                tokens.refresh_expires_at,
            )
            .await?;
        Ok::<_, BrokerError>(connection)
    }
    .await;

    match outcome {
        Ok(connection) => {
            state
                .store
                .try_complete_session(&session.id, &connection.id)
                .await?;
        }
        Err(err) => {
            state
                .store
                .mark_session_failed(&session.id, "TOKEN_EXCHANGE_FAILED", &err.to_string())
                .await?;
        }
    }
    Ok(())
}

// --- GET /v1/auth/sessions/:id --------------------------------------------

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum SessionStatusView {
    Pending,
    Completed { connection: ConnectionView },
    Failed { code: String, message: String },
    Expired,
}

async fn get_session_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionStatusView>, BrokerError> {
    let session = state
        .store
        .get_session(&id)
        .await?
        .ok_or(BrokerError::SessionNotFound)?;

    if session.status == SessionStatus::Pending && session.is_expired(Utc::now()) {
        return Ok(Json(SessionStatusView::Expired));
    }

    Ok(Json(match session.status {
        SessionStatus::Pending => SessionStatusView::Pending,
        SessionStatus::Expired => SessionStatusView::Expired,
        SessionStatus::Failed => SessionStatusView::Failed {
            code: session.error_code.unwrap_or_else(|| "UNKNOWN".to_string()),
            message: session.error_message.unwrap_or_default(),
        },
        SessionStatus::Completed => {
            let connection_id = session.connection_id.ok_or(BrokerError::Internal)?;
            let connection = state
                .store
                .get_connection(&connection_id)
                .await?
                .ok_or(BrokerError::ConnectionNotFound)?;
            SessionStatusView::Completed {
                connection: ConnectionView::from(&connection),
            }
        }
    }))
}

// --- Connections ------------------------------------------------------------

async fn refresh_connection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConnectionView>, BrokerError> {
    let connection = state
        .store
        .get_connection(&id)
        .await?
        .ok_or(BrokerError::ConnectionNotFound)?;
    if connection.revoked_at.is_some() {
        return Err(BrokerError::TokenRevoked);
    }
    let refresh_token = connection
        .refresh_token_encrypted
        .as_deref()
        .map(|t| state.cipher.decrypt(t))
        .transpose()?
        .ok_or(BrokerError::TokenRevoked)?;

    let tokens = match connection.platform.as_str() {
        "tiktok" => {
            state
                .tiktok
                .clone()
                .ok_or(BrokerError::ProviderNotConfigured)?
                .refresh(&refresh_token)
                .await?
        }
        "kwai" => {
            state
                .kwai
                .clone()
                .ok_or(BrokerError::ProviderNotConfigured)?
                .refresh(&refresh_token)
                .await?
        }
        _ => return Err(BrokerError::BadRequest("unknown platform".to_string())),
    };

    // Section 19/40: a rotated refresh token fully replaces the old one;
    // an omitted one means the old one is still valid — never discarded.
    let access_encrypted = state.cipher.encrypt(&tokens.access_token)?;
    let refresh_encrypted = tokens
        .refresh_token
        .as_deref()
        .map(|t| state.cipher.encrypt(t))
        .transpose()?;
    state
        .store
        .update_connection_tokens(
            &id,
            &access_encrypted,
            refresh_encrypted.as_deref(),
            tokens.access_expires_at,
            tokens.refresh_expires_at,
        )
        .await?;

    let refreshed = state
        .store
        .get_connection(&id)
        .await?
        .ok_or(BrokerError::ConnectionNotFound)?;
    Ok(Json(ConnectionView::from(&refreshed)))
}

async fn revoke_connection(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, BrokerError> {
    let connection = state
        .store
        .get_connection(&id)
        .await?
        .ok_or(BrokerError::ConnectionNotFound)?;

    if let Some(access_encrypted) = Some(&connection.access_token_encrypted) {
        if let Ok(access_token) = state.cipher.decrypt(access_encrypted) {
            let _ = match connection.platform.as_str() {
                "tiktok" => match &state.tiktok {
                    Some(c) => c.revoke(&access_token).await,
                    None => Ok(()),
                },
                "kwai" => match &state.kwai {
                    Some(c) => c.revoke(&access_token).await,
                    None => Ok(()),
                },
                _ => Ok(()),
            };
        }
    }

    state.store.revoke_connection(&id).await?;
    Ok(Json(serde_json::json!({ "revoked": true })))
}

#[derive(Serialize)]
struct ConnectionStatusView {
    connection_id: String,
    platform: String,
    revoked: bool,
    access_expires_at: Option<DateTime<Utc>>,
    refresh_expires_at: Option<DateTime<Utc>>,
}

async fn connection_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ConnectionStatusView>, BrokerError> {
    let connection = state
        .store
        .get_connection(&id)
        .await?
        .ok_or(BrokerError::ConnectionNotFound)?;
    Ok(Json(ConnectionStatusView {
        connection_id: connection.id,
        platform: connection.platform,
        revoked: connection.revoked_at.is_some(),
        access_expires_at: connection.access_expires_at,
        refresh_expires_at: connection.refresh_expires_at,
    }))
}
