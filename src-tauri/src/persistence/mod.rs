//! Persistence: SQLite connection management and migration bootstrap only.
//! Table-specific query logic belongs in `infrastructure::repositories`,
//! which implements the `domain::ports::repositories` traits.

pub mod db;
