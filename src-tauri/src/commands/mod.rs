//! Commands: the thin Tauri IPC boundary. Every command here does nothing
//! but extract `State<AppState>`, call exactly one application-layer
//! service method, and map the result — no business logic and no SQL
//! belongs in this module (Rules 1 and 2).

pub mod activity_commands;
pub mod channel_commands;
pub mod content_commands;
pub mod import_commands;
pub mod media_protocol;
pub mod notification_commands;
pub mod settings_commands;
pub mod source_commands;
pub mod system_commands;
pub mod workspace_commands;
