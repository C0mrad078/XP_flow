//! Application: use-case orchestration. Commands call these services rather
//! than touching repositories directly, keeping the IPC boundary thin and
//! the business rules testable without Tauri in the loop.

pub mod activity_service;
pub mod channel_service;
pub mod content_service;
pub mod media_ingestion_service;
pub mod settings_service;
pub mod source_service;
pub mod workspace_service;
