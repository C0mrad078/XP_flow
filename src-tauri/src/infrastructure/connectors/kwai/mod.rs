//! Kwai: entirely broker-owned authorization (section 17-19) — the
//! desktop only opens the browser and polls for completion.

pub mod auth_provider;
pub mod connector;

pub use auth_provider::KwaiAuthProvider;
pub use connector::KwaiConnector;
