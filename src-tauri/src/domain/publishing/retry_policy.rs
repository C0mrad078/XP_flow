use chrono::{DateTime, Duration, Utc};
use rand::Rng;

/// After this many failed attempts, a publication stops retrying
/// automatically and needs a manual "Retry" (section 75).
pub const MAX_PUBLISH_ATTEMPTS: i32 = 6;

/// Base backoff per attempt number (section 73's "30 sec, 2 min, 10 min,
/// 30 min..." example), centralized here rather than hardcoded inside
/// any individual provider client.
fn base_backoff_seconds(attempt_number: i32) -> i64 {
    match attempt_number {
        ..=1 => 30,
        2 => 120,
        3 => 600,
        4 => 1800,
        _ => 3600,
    }
}

/// When the next attempt should run, given the attempt number that just
/// failed and (if the provider supplied one) its own `Retry-After`
/// (section 74 — provider-specific rate metadata always overrides the
/// generic policy). Jittered ±20% so a burst of publications that fail
/// for the same reason at the same time don't all retry in lockstep.
pub fn next_retry_at(
    attempt_number: i32,
    now: DateTime<Utc>,
    provider_retry_after_seconds: Option<u64>,
) -> DateTime<Utc> {
    if let Some(seconds) = provider_retry_after_seconds {
        return now + Duration::seconds(seconds as i64);
    }

    let base = base_backoff_seconds(attempt_number);
    let jitter_fraction = rand::thread_rng().gen_range(-0.2..=0.2);
    let jittered = (base as f64 * (1.0 + jitter_fraction)).round() as i64;
    now + Duration::seconds(jittered.max(1))
}

/// Whether `attempt_number` (the attempt that just failed) has exhausted
/// the retry budget — the caller still checks `PublishError::is_retryable`
/// separately; this only bounds *how many times*, not *whether at all*.
pub fn attempts_exhausted(attempt_number: i32) -> bool {
    attempt_number >= MAX_PUBLISH_ATTEMPTS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_with_attempt_number() {
        let now = Utc::now();
        // Run many times since jitter is randomized — every sample must
        // still respect the ordering between attempt tiers.
        for _ in 0..50 {
            let first = next_retry_at(1, now, None) - now;
            let third = next_retry_at(3, now, None) - now;
            assert!(third > first);
        }
    }

    #[test]
    fn provider_retry_after_always_overrides_the_generic_policy() {
        let now = Utc::now();
        let retry_at = next_retry_at(1, now, Some(3600));
        assert_eq!(retry_at, now + Duration::seconds(3600));
    }

    #[test]
    fn attempts_exhausted_respects_the_budget() {
        assert!(!attempts_exhausted(MAX_PUBLISH_ATTEMPTS - 1));
        assert!(attempts_exhausted(MAX_PUBLISH_ATTEMPTS));
        assert!(attempts_exhausted(MAX_PUBLISH_ATTEMPTS + 1));
    }
}
