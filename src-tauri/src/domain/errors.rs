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
}

pub type DomainResult<T> = Result<T, DomainError>;
