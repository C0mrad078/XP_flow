//! Services: cross-cutting technical services that support the application
//! layer but are not themselves use-case orchestration for a single
//! aggregate (media toolchain detection, notification fan-out).

pub mod media_status_service;
pub mod notification_service;
