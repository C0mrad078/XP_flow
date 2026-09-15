//! Shared OAuth security primitives (section 7 of the Phase 4 brief):
//! state-token/PKCE generation and the in-memory authorization-session
//! model every provider strategy builds on. Provider-specific behavior
//! (endpoints, token mapping, scope sets) lives in
//! `infrastructure::auth::<provider>`, not here.

pub mod flow_state;
pub mod pkce;
pub mod session;

pub use flow_state::AuthFlowState;
pub use pkce::{
    challenge_from_verifier, generate_pkce, GooglePkceStrategy, Pkce, TikTokPkceStrategy,
};
pub use session::{generate_state, AuthSession, AUTH_SESSION_TTL};
