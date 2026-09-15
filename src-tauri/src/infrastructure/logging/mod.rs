use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, EnvFilter};

/// Initializes structured logging: pretty output to the dev console, plus a
/// rolling-daily JSON file under `<data_dir>/logs`. The returned
/// `WorkerGuard` must be kept alive for the process lifetime (dropping it
/// flushes and stops the background writer) — callers store it in
/// `AppState`.
///
/// Level defaults to `info` and can be overridden with `RUST_LOG` (Rule:
/// never log secrets — no credential, token or password value should ever
/// reach a `tracing::*!` call site).
pub fn init(log_dir: &Path) -> WorkerGuard {
    let file_appender = tracing_appender::rolling::daily(log_dir, "xpflow.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = fmt::layer()
        .json()
        .with_writer(non_blocking)
        .with_ansi(false);

    let console_layer = fmt::layer().with_target(false).with_ansi(true);

    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    tracing_subscriber::registry()
        .with(env_filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
}
