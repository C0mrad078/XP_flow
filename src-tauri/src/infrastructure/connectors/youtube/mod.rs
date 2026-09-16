//! YouTube: real OAuth 2.0 installed-app + PKCE flow, talking directly to
//! Google — no Auth Broker (section 5). Split per section 36: `config`
//! (non-secret settings), `api_client` (raw Google HTTP transport),
//! `auth_provider` (drives a new authorization attempt), `connector`
//! (validate/refresh/disconnect/get_profile for an already-connected
//! account).

pub mod api_client;
pub mod auth_provider;
pub mod config;
pub mod connector;
pub mod uploader;

pub use auth_provider::YouTubeAuthProvider;
pub use config::YouTubeAuthConfig;
pub use connector::YouTubeConnector;
pub use uploader::YouTubeUploader;
