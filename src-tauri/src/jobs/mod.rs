//! Job foundation (section 44/45): the vocabulary a future local job
//! runner will use. No scheduler or worker pool exists yet — see
//! `docs/architecture.md` for what's deferred to Phase 2.

pub mod job;

pub use job::{Job, JobStatus, JobType};
