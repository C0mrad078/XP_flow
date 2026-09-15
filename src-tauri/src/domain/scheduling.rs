//! Pure scheduling algorithm: given a channel's recurring slots, date
//! exceptions and workspace timezone, find the next open publishing time.
//!
//! Deliberately pure (no repository/clock access) so it can be tested
//! exhaustively — including DST transitions — without a database or a real
//! system clock (section 20). The caller (`application::scheduler_service`)
//! is responsible for fetching slots/exceptions/already-booked instants and
//! for persisting the result inside a transaction.

use std::collections::HashSet;

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use chrono_tz::Tz;

use super::platform::Platform;
use super::schedule_exception::{ScheduleException, ScheduleExceptionKind};
use super::schedule_slot::ScheduleSlot;

/// How many calendar days ahead to search before giving up. A channel with
/// no active slots (or only slots that keep landing on exception days)
/// would otherwise search forever.
pub const DEFAULT_SEARCH_HORIZON_DAYS: i64 = 120;

/// The slots that actually apply to `platform` on `weekday` (section 17):
/// platform-specific slots take priority; channel-default (`platform:
/// None`) slots only apply when there is no platform-specific slot for
/// that exact weekday.
pub fn effective_slots_for_weekday(
    slots: &[ScheduleSlot],
    weekday: i32,
    platform: Platform,
) -> Vec<&ScheduleSlot> {
    let platform_specific: Vec<&ScheduleSlot> = slots
        .iter()
        .filter(|s| s.is_active && s.day_of_week == weekday && s.platform == Some(platform))
        .collect();

    if !platform_specific.is_empty() {
        return platform_specific;
    }

    slots
        .iter()
        .filter(|s| s.is_active && s.day_of_week == weekday && s.platform.is_none())
        .collect()
}

fn is_skipped(exceptions: &[ScheduleException], date: NaiveDate) -> bool {
    exceptions
        .iter()
        .any(|e| e.date == date && e.kind == ScheduleExceptionKind::Skip)
}

/// Resolves a slot's local wall-clock time on `date` to a UTC instant.
/// DST-safe: a time that falls in a "spring forward" gap has no valid
/// mapping and is skipped by the caller; a time that falls in a "fall
/// back" ambiguous window deterministically resolves to the *earlier* of
/// the two instants.
fn resolve_local_time(tz: Tz, date: NaiveDate, time_of_day: &str) -> Option<DateTime<Utc>> {
    let (hour, minute) = ScheduleSlot::parse_time_of_day(time_of_day).ok()?;
    let naive = date.and_hms_opt(hour, minute, 0)?;
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Some(dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(earliest, _latest) => Some(earliest.with_timezone(&Utc)),
        chrono::LocalResult::None => None,
    }
}

/// Finds the earliest UTC instant at or after `earliest` that:
/// - falls on a weekday/time covered by one of the channel's active slots
///   for `platform` (section 14-17),
/// - is not on a date with a `Skip` exception (section 17),
/// - is not already present in `taken` (section 27/28 — the caller
///   supplies every already-`Scheduled` instant for this channel so the
///   algorithm itself never proposes a collision; the database unique
///   index is the final backstop beneath this).
///
/// Returns `None` if nothing is found within `horizon_days`.
pub fn find_next_available_slot(
    slots: &[ScheduleSlot],
    exceptions: &[ScheduleException],
    timezone: &str,
    platform: Platform,
    earliest: DateTime<Utc>,
    taken: &HashSet<DateTime<Utc>>,
    horizon_days: i64,
) -> Option<DateTime<Utc>> {
    let tz: Tz = timezone.parse().ok()?;
    let start_date = earliest.with_timezone(&tz).date_naive();

    for offset in 0..horizon_days {
        let date = start_date + Duration::days(offset);
        if is_skipped(exceptions, date) {
            continue;
        }

        let weekday = iso_weekday(date);
        let mut candidates: Vec<&ScheduleSlot> =
            effective_slots_for_weekday(slots, weekday, platform);
        candidates.sort_by(|a, b| a.time_of_day.cmp(&b.time_of_day));

        for slot in candidates {
            let Some(candidate_utc) = resolve_local_time(tz, date, &slot.time_of_day) else {
                continue;
            };
            if candidate_utc < earliest {
                continue;
            }
            if taken.contains(&candidate_utc) {
                continue;
            }
            return Some(candidate_utc);
        }
    }

    None
}

/// 0 = Monday .. 6 = Sunday, matching `ScheduleSlot::day_of_week`.
fn iso_weekday(date: NaiveDate) -> i32 {
    date.weekday().num_days_from_monday() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn slot(platform: Option<Platform>, day: i32, time: &str) -> ScheduleSlot {
        ScheduleSlot::new(Uuid::new_v4(), platform, day, time).unwrap()
    }

    #[test]
    fn platform_specific_slot_wins_over_channel_default_on_same_weekday() {
        let slots = vec![
            slot(None, 1, "09:00"),
            slot(Some(Platform::TikTok), 1, "18:00"),
        ];
        let effective = effective_slots_for_weekday(&slots, 1, Platform::TikTok);
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].time_of_day, "18:00");
    }

    #[test]
    fn channel_default_applies_when_no_platform_specific_slot_exists() {
        let slots = vec![slot(None, 1, "09:00")];
        let effective = effective_slots_for_weekday(&slots, 1, Platform::YouTube);
        assert_eq!(effective.len(), 1);
        assert_eq!(effective[0].time_of_day, "09:00");
    }

    #[test]
    fn finds_the_next_matching_weekday_and_time_in_utc() {
        // Tuesday 2024-01-02 is a known Tuesday (day_of_week = 1).
        let slots = vec![slot(None, 1, "10:00")];
        let earliest = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(); // Monday
        let found = find_next_available_slot(
            &slots,
            &[],
            "UTC",
            Platform::YouTube,
            earliest,
            &HashSet::new(),
            30,
        )
        .unwrap();
        assert_eq!(found, Utc.with_ymd_and_hms(2024, 1, 2, 10, 0, 0).unwrap());
    }

    #[test]
    fn skips_taken_instants_and_moves_to_the_following_week() {
        let slots = vec![slot(None, 1, "10:00")];
        let first = Utc.with_ymd_and_hms(2024, 1, 2, 10, 0, 0).unwrap();
        let mut taken = HashSet::new();
        taken.insert(first);

        let found = find_next_available_slot(
            &slots,
            &[],
            "UTC",
            Platform::YouTube,
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            &taken,
            30,
        )
        .unwrap();
        assert_eq!(found, Utc.with_ymd_and_hms(2024, 1, 9, 10, 0, 0).unwrap());
    }

    #[test]
    fn respects_skip_exceptions() {
        let slots = vec![slot(None, 1, "10:00")];
        let exceptions = vec![ScheduleException::new_skip(
            Uuid::new_v4(),
            NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
            None,
        )];

        let found = find_next_available_slot(
            &slots,
            &exceptions,
            "UTC",
            Platform::YouTube,
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            &HashSet::new(),
            30,
        )
        .unwrap();
        // The following Tuesday, since 2024-01-02 is skipped.
        assert_eq!(found, Utc.with_ymd_and_hms(2024, 1, 9, 10, 0, 0).unwrap());
    }

    #[test]
    fn returns_none_when_no_slots_exist() {
        let found = find_next_available_slot(
            &[],
            &[],
            "UTC",
            Platform::YouTube,
            Utc::now(),
            &HashSet::new(),
            30,
        );
        assert!(found.is_none());
    }

    #[test]
    fn is_dst_safe_across_the_spring_forward_gap() {
        // America/New_York springs forward on 2024-03-10: 02:00 -> 03:00
        // local time never exists that day. A slot requesting 02:30 must be
        // skipped entirely for that date, not silently shifted.
        let slots = vec![slot(None, 6, "02:30")]; // Sunday
        let earliest = Utc.with_ymd_and_hms(2024, 3, 9, 0, 0, 0).unwrap();
        let found = find_next_available_slot(
            &slots,
            &[],
            "America/New_York",
            Platform::YouTube,
            earliest,
            &HashSet::new(),
            30,
        )
        .unwrap();
        // Must skip 2024-03-10 (the gap) and land on the following Sunday.
        assert_eq!(
            found
                .with_timezone(&chrono_tz::America::New_York)
                .date_naive(),
            NaiveDate::from_ymd_opt(2024, 3, 17).unwrap()
        );
    }

    #[test]
    fn is_dst_safe_across_the_fall_back_ambiguous_window_and_picks_the_earlier_instant() {
        // America/New_York falls back on 2024-11-03: 01:30 local occurs
        // twice (once at -04:00, once at -05:00). We deterministically
        // pick the earlier (still-DST) instant.
        let slots = vec![slot(None, 6, "01:30")]; // Sunday
        let earliest = Utc.with_ymd_and_hms(2024, 11, 2, 0, 0, 0).unwrap();
        let found = find_next_available_slot(
            &slots,
            &[],
            "America/New_York",
            Platform::YouTube,
            earliest,
            &HashSet::new(),
            30,
        )
        .unwrap();

        let expected_earliest = chrono_tz::America::New_York
            .with_ymd_and_hms(2024, 11, 3, 1, 30, 0)
            .earliest()
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(found, expected_earliest);
    }

    #[test]
    fn returns_none_past_the_search_horizon() {
        // Slot only fires on a weekday that never occurs within the tiny
        // horizon we give it starting from a day already past it.
        let slots = vec![slot(None, 1, "10:00")];
        let earliest = Utc.with_ymd_and_hms(2024, 1, 3, 0, 0, 0).unwrap(); // Wednesday
        let found = find_next_available_slot(
            &slots,
            &[],
            "UTC",
            Platform::YouTube,
            earliest,
            &HashSet::new(),
            3,
        );
        assert!(found.is_none());
    }
}
