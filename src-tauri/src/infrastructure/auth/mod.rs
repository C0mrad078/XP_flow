//! Shared authentication infrastructure: the loopback OAuth callback
//! listener, the Auth Broker HTTP client, HTTP client policy (timeouts),
//! and secret redaction. Provider-specific behavior lives in
//! `infrastructure::connectors::{youtube,tiktok,kwai}`.

pub mod broker_client;
pub mod broker_config;
pub mod http_client;
pub mod loopback_listener;
pub mod redact;

pub use broker_client::BrokerClient;
pub use broker_config::AuthBrokerConfig;
pub use loopback_listener::LoopbackListener;
