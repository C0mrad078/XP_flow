use thiserror::Error;

/// Errors that can originate from the domain layer.
///
/// Domain errors never carry infrastructure types (no `sqlx::Error`, no
/// `std::io::Error`, ...). Infrastructure adapters translate their own
/// failures into these variants before returning across the domain boundary.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("{entity} {id} was not found")]
    NotFound { entity: &'static str, id: String },

    #[error("invalid state transition for {entity}: {from} -> {to}")]
    InvalidTransition {
        entity: &'static str,
        from: String,
        to: String,
    },

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("repository failure: {0}")]
    Repository(String),

    #[error("invalid value for {field}: {reason}")]
    InvalidValue {
        field: &'static str,
        reason: String,
    },

    /// A publication already exists for this (video, channel, platform)
    /// triple in a non-terminal state (section 50/51) — enforced first here
    /// and backstopped by a DB partial unique index.
    #[error("a publication already exists for this video on this channel/platform")]
    PublicationAlreadyExists,

    /// The requested slot/time is already claimed by another scheduled
    /// publication (section 27/28) — enforced first here and backstopped by
    /// a DB partial unique index.
    #[error("that time slot is already scheduled")]
    ScheduleConflict,

    #[error("channel {channel_id} is paused and cannot be scheduled to")]
    ChannelPaused { channel_id: String },

    #[error("channel {channel_id} has no active schedule slots")]
    ChannelHasNoSchedule { channel_id: String },

    #[error("publication {publication_id} is locked and cannot be auto-rescheduled")]
    PublicationLocked { publication_id: String },

    #[error("no available slot could be found within the search window")]
    NoAvailableSlot,

    #[error("bulk schedule operation failed: {0}")]
    BulkScheduleFailed(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
