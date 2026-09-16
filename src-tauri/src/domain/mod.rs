//! Domain layer: entities, value objects and ports. This module must never
//! import from `infrastructure`, `persistence`, `platform`, `commands` or
//! any UI-facing code (Rule 3/4 of the Phase 1 architecture rules).

pub mod activity_event;
pub mod app_settings;
pub mod auth_error;
pub mod capability;
pub mod channel;
pub mod connection_health;
pub mod duplicate_match;
pub mod errors;
pub mod media_error;
pub mod notification;
pub mod oauth;
pub mod platform;
pub mod platform_account;
pub mod ports;
pub mod provider_identity;
pub mod publication;
pub mod publication_query;
pub mod publishing;
pub mod queue_item;
pub mod readiness;
pub mod schedule_exception;
pub mod schedule_slot;
pub mod scheduling;
pub mod video;
pub mod video_query;
pub mod video_source;
pub mod video_status;
pub mod workspace;
