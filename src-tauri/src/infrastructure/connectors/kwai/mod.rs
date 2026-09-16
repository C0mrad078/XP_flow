//! Kwai: entirely broker-owned authorization (section 17-19) — the
//! desktop only opens the browser and polls for completion.

pub mod auth_provider;
pub mod config;
pub mod connector;
pub mod uploader;

pub use auth_provider::KwaiAuthProvider;
pub use config::KwaiPublishConfig;
pub use connector::KwaiConnector;
pub use uploader::KwaiUploader;
