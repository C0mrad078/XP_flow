//! Application: use-case orchestration. Commands call these services rather
//! than touching repositories directly, keeping the IPC boundary thin and
//! the business rules testable without Tauri in the loop.

pub mod activity_service;
pub mod channel_service;
pub mod content_service;
pub mod credential_acquisition_service;
pub mod media_ingestion_service;
pub mod metadata_template_service;
pub mod platform_account_service;
pub mod platform_auth_service;
pub mod publication_service;
pub mod publishing_engine_service;
pub mod publishing_readiness_service;
pub mod schedule_slot_service;
pub mod scheduler_service;
pub mod settings_service;
pub mod source_service;
pub mod token_lifecycle_service;
pub mod workspace_service;
