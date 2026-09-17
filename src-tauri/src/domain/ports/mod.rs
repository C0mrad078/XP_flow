//! Ports: trait contracts the domain/application layers depend on.
//! Implementations live in `infrastructure` and `platform` (Rule 4 —
//! infrastructure may depend on domain abstractions, never the reverse).

pub mod hashing;
pub mod media_service;
pub mod platform_auth_provider;
pub mod platform_connector;
pub mod platform_publisher;
pub mod progress_publisher;
pub mod repositories;
pub mod secure_storage;
