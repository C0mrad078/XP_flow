//! Ports: trait contracts the domain/application layers depend on.
//! Implementations live in `infrastructure` and `platform` (Rule 4 —
//! infrastructure may depend on domain abstractions, never the reverse).

pub mod media_service;
pub mod platform_connector;
pub mod repositories;
pub mod secure_storage;
