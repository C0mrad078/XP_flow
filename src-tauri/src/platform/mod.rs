//! Platform: the only place allowed to know about OS-specific behavior
//! (data directory layout, OS credential stores). Everything else in the
//! backend depends on the types exposed here, never on `cfg(target_os)`
//! directly (Rule 5).

pub mod paths;
pub mod secure_storage_keyring;
