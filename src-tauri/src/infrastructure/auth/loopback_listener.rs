use std::collections::HashMap;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::domain::auth_error::AuthError;

/// The query parameters a provider's OAuth redirect can carry back.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct CallbackParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// A short-lived, single-use loopback HTTP server for an OAuth redirect
/// (sections 7/10). Binds **only** to `127.0.0.1` on an OS-assigned port —
/// never `0.0.0.0` — accepts exactly one callback request, then shuts
/// down. Used by both YouTube (which exchanges the code directly) and
/// TikTok (which forwards the code to the Auth Broker for exchange) —
/// this listener only ever captures the redirect, it never knows or cares
/// what happens to the code afterward.
pub struct LoopbackListener {
    pub redirect_uri: String,
    receiver: oneshot::Receiver<CallbackParams>,
    shutdown_tx: oneshot::Sender<()>,
    join_handle: tokio::task::JoinHandle<()>,
}

const CALLBACK_PATH_PREFIX: &str = "/oauth";

impl LoopbackListener {
    /// Starts listening and returns immediately with the `redirect_uri`
    /// the authorize URL should use — `provider` becomes the path segment
    /// (e.g. `youtube` -> `http://127.0.0.1:<port>/oauth/youtube/callback`,
    /// matching section 7's example).
    pub async fn start(provider: &str) -> Result<Self, AuthError> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await.map_err(|e| {
            AuthError::TokenExchangeFailed {
                detail: format!("failed to bind loopback listener: {e}"),
            }
        })?;
        let port = listener
            .local_addr()
            .map_err(|e| AuthError::TokenExchangeFailed {
                detail: format!("failed to read loopback listener port: {e}"),
            })?
            .port();
        let path = format!("{CALLBACK_PATH_PREFIX}/{provider}/callback");
        let redirect_uri = format!("http://127.0.0.1:{port}{path}");

        let (result_tx, result_rx) = oneshot::channel::<CallbackParams>();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let app_path = path.clone();
        let app = Router::new()
            .route(&app_path, get(handle_callback))
            .with_state(std::sync::Arc::new(tokio::sync::Mutex::new(Some(
                result_tx,
            ))));

        let join_handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await;
        });

        Ok(Self {
            redirect_uri,
            receiver: result_rx,
            shutdown_tx,
            join_handle,
        })
    }

    /// Waits for the single expected callback (or `cancel`, or `timeout`),
    /// then always tears the listener down (section 7: "shut down after
    /// completion; never remain permanently exposed").
    pub async fn wait_for_callback(
        self,
        expected_state: &str,
        mut cancel: oneshot::Receiver<()>,
        timeout: Duration,
    ) -> Result<String, AuthError> {
        let outcome = tokio::select! {
            received = self.receiver => received.map_err(|_| AuthError::AuthTimeout),
            _ = &mut cancel => Err(AuthError::AuthCancelled),
            _ = tokio::time::sleep(timeout) => Err(AuthError::AuthTimeout),
        };

        let _ = self.shutdown_tx.send(());
        let _ = self.join_handle.await;

        let params = outcome?;
        if let Some(error) = params.error {
            return Err(if error == "access_denied" {
                AuthError::AuthCancelled
            } else {
                AuthError::TokenExchangeFailed {
                    detail: format!(
                        "provider returned error: {error}{}",
                        params
                            .error_description
                            .map(|description| format!(" ({description})"))
                            .unwrap_or_default()
                    ),
                }
            });
        }
        let state = params.state.ok_or(AuthError::AuthStateMismatch)?;
        if state != expected_state {
            return Err(AuthError::AuthStateMismatch);
        }
        params.code.ok_or(AuthError::AuthCodeInvalid)
    }
}

type SenderSlot = std::sync::Arc<tokio::sync::Mutex<Option<oneshot::Sender<CallbackParams>>>>;

const SUCCESS_PAGE: &str =
    "<html><body style=\"font-family:sans-serif;text-align:center;padding-top:4rem\">\
     <h2>XP FLOW</h2><p>Account connected successfully.</p><p>You can return to XP FLOW.</p>\
     </body></html>";

const ERROR_PAGE: &str =
    "<html><body style=\"font-family:sans-serif;text-align:center;padding-top:4rem\">\
     <h2>XP FLOW</h2><p>XP FLOW could not connect your account.</p>\
     <p>You can close this window and return to XP FLOW.</p>\
     </body></html>";

/// Never echoes the raw callback back to the browser (section 19/20: no
/// authorization code, token or internal state in the page a stranger
/// glancing at the user's screen could see) — the page only ever says
/// "success" or "not success," in general terms.
async fn handle_callback(
    State(sender): State<SenderSlot>,
    Query(params): Query<HashMap<String, String>>,
) -> Html<&'static str> {
    let succeeded = params.contains_key("code") && !params.contains_key("error");
    let callback = CallbackParams {
        code: params.get("code").cloned(),
        state: params.get("state").cloned(),
        error: params.get("error").cloned(),
        error_description: params.get("error_description").cloned(),
    };
    if let Some(tx) = sender.lock().await.take() {
        let _ = tx.send(callback);
    }
    Html(if succeeded { SUCCESS_PAGE } else { ERROR_PAGE })
}
