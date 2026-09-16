//! Infrastructure: concrete adapters for the ports declared in
//! `domain::ports`. May depend on `domain`; nothing in `domain` may depend
//! on this module (Rule 4).

pub mod auth;
pub mod connectors;
pub mod filesystem;
pub mod hashing;
pub mod logging;
pub mod media;
pub mod publishing;
pub mod repositories;
pub mod watcher;
