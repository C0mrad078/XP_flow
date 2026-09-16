//! TikTok: desktop-driven authorize + loopback callback, broker-only
//! confidential token exchange/refresh/revoke (section 10-14).

pub mod auth_provider;
pub mod config;
pub mod connector;
pub mod uploader;

pub use auth_provider::TikTokAuthProvider;
pub use config::TikTokAuthConfig;
pub use connector::TikTokConnector;
pub use uploader::TikTokUploader;
