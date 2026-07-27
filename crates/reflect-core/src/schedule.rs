//! When Reflect's daily reminder is due, and when it last went out.

use std::io;
use std::path::PathBuf;

use chrono::{Days, NaiveDateTime, NaiveTime, TimeDelta};

/// The time Reflect nudges you on, until Settings lets the user move it.
/// Evening — late enough that there's a day behind you worth looking back on.
pub const DEFAULT_DAILY_TIME: NaiveTime = match NaiveTime::from_hms_opt(21, 0, 0) {
    Some(time) => time,
    None => unreachable!(),
};

/// How close to the next day's reminder a missed one stops being worth firing.
///
/// Inclusive: a catch-up exactly this far ahead of tomorrow's reminder is let
/// go rather than shown almost on top of it.
pub const CATCH_UP_CUTOFF: TimeDelta = TimeDelta::hours(1);

/// The one reminder a day, at the time the user picked.
///
/// Every instant here is naive local wall-clock time — the time the user would
/// read off a clock on the wall, which is also the time they set. That is what
/// makes daylight saving come out right with no mechanism for it: in spring the
/// reminder fires when the wall clock passes the time, and in autumn the hour
/// that happens twice can't fire twice, because by its second pass that
/// occurrence is already recorded as delivered.
pub struct Schedule {
    daily_time: NaiveTime,
}

/// What [`Schedule::due`] decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reminder {
    /// Show the reminder now.
    ///
    /// `occurrence` is the scheduled time it belongs to, not the moment it
    /// actually appears — a catch-up on Tuesday morning is still Monday
    /// evening's reminder, and that is what should be recorded as delivered.
    Due { occurrence: NaiveDateTime },
    /// Nothing owed. Ask again later.
    NotDue,
}

impl Schedule {
    /// A reminder every day at `daily_time`.
    pub const fn daily_at(daily_time: NaiveTime) -> Self {
        Self { daily_time }
    }

    /// The most recent scheduled time at or before `now` — the occurrence
    /// `now` is living inside.
    pub fn occurrence_on_or_before(&self, now: NaiveDateTime) -> NaiveDateTime {
        let today = now.date().and_time(self.daily_time);
        if today <= now {
            return today;
        }
        // Before today's time, so the current occurrence is yesterday's. The
        // fallback only bites at the very first representable date, where
        // there is no yesterday to reach for.
        now.date()
            .pred_opt()
            .map_or(today, |yesterday| yesterday.and_time(self.daily_time))
    }

    /// Whether to nudge the user right now, given when they were last nudged.
    ///
    /// Reflect isn't running a timer so much as answering this question over
    /// and over, which is what lets a machine that was asleep at the appointed
    /// hour still get its reminder: it wakes, asks, and finds one owed.
    pub fn due(&self, now: NaiveDateTime, last_reminded: NaiveDateTime) -> Reminder {
        let occurrence = self.occurrence_on_or_before(now);

        // Already delivered. This is also what makes a dismissed notification
        // the end of it for that day: Reflect records that it nudged, never
        // whether the nudge was taken up.
        if last_reminded >= occurrence {
            return Reminder::NotDue;
        }

        // Late enough that tomorrow's is nearly here. Two reminders almost
        // back to back is worse than a missed one, so this day is let go and
        // the next occurrence fires normally.
        let next = occurrence + Days::new(1);
        if next - now <= CATCH_UP_CUTOFF {
            return Reminder::NotDue;
        }

        Reminder::Due { occurrence }
    }
}

/// The file remembering which occurrence Reflect last nudged on, so a restart
/// doesn't nudge again for a day already done.
///
/// A single wall-clock timestamp in a plain text file, next to the entries and
/// as readable as they are.
pub struct LastReminder {
    path: PathBuf,
}

/// How the timestamp is written — sortable, and legible to anyone who opens
/// the file to see what Reflect thinks it did.
const TIMESTAMP: &str = "%Y-%m-%dT%H:%M:%S";

impl LastReminder {
    /// The record kept at `path`. The file needn't exist yet.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// When Reflect last nudged, starting the record if there isn't one.
    ///
    /// A missing, unreadable, or impossible record all resolve the same way:
    /// to the occurrence `now` sits in, as though it had just been delivered.
    /// That is deliberately not a catch-up — installing Reflect at ten past
    /// nine shouldn't fire a notification a second later, and a clock that has
    /// been moved back shouldn't leave Reflect silent until real time catches
    /// up with a timestamp from its future.
    pub fn load_or_start(&self, schedule: &Schedule, now: NaiveDateTime) -> io::Result<NaiveDateTime> {
        if let Some(recorded) = self.read()? {
            if recorded <= now {
                return Ok(recorded);
            }
        }

        let restarted_at = schedule.occurrence_on_or_before(now);
        self.record(restarted_at)?;
        Ok(restarted_at)
    }

    /// Remember that `occurrence`'s reminder went out.
    pub fn record(&self, occurrence: NaiveDateTime) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, format!("{}\n", occurrence.format(TIMESTAMP)))
    }

    fn read(&self) -> io::Result<Option<NaiveDateTime>> {
        let text = match std::fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err),
        };
        Ok(NaiveDateTime::parse_from_str(text.trim(), TIMESTAMP).ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day;

    /// Reflect's reminder time in these tests: nine in the evening.
    const NINE_PM: NaiveTime = DEFAULT_DAILY_TIME;

    fn schedule() -> Schedule {
        Schedule::daily_at(NINE_PM)
    }

    /// A wall-clock instant on the 24th of July 2026, written the way a clock
    /// would show it.
    fn at(hour: u32, minute: u32) -> NaiveDateTime {
        on(24, hour, minute)
    }

    /// The same, on any day of that July.
    fn on(date: u32, hour: u32, minute: u32) -> NaiveDateTime {
        day(2026, 7, date)
            .and_hms_opt(hour, minute, 0)
            .expect("test clock time must be a real time")
    }

    #[test]
    fn the_reminder_fires_when_the_clock_reaches_the_appointed_hour() {
        let last_night = on(23, 21, 0);

        assert_eq!(
            schedule().due(at(21, 0), last_night),
            Reminder::Due {
                occurrence: at(21, 0)
            }
        );
    }

    #[test]
    fn nothing_fires_before_the_appointed_hour() {
        let last_night = on(23, 21, 0);

        assert_eq!(schedule().due(at(20, 59), last_night), Reminder::NotDue);
    }

    #[test]
    fn a_machine_asleep_at_the_hour_gets_its_reminder_when_it_wakes() {
        // Asleep from before nine, woken at midday the next day. The reminder
        // owed is still the previous evening's.
        let last_reminded = on(23, 21, 0);

        assert_eq!(
            schedule().due(on(25, 12, 0), last_reminded),
            Reminder::Due {
                occurrence: on(24, 21, 0)
            }
        );
    }

    #[test]
    fn a_catch_up_is_let_go_when_the_next_reminder_is_nearly_here() {
        // Woken at half eight in the evening, with the day's own reminder due
        // at nine. Firing the missed one now would put two of them half an
        // hour apart.
        let last_reminded = on(23, 21, 0);

        assert_eq!(schedule().due(on(25, 20, 30), last_reminded), Reminder::NotDue);
    }

    #[test]
    fn the_reminder_let_go_is_only_that_one_and_the_next_still_fires() {
        let last_reminded = on(23, 21, 0);
        let schedule = schedule();

        // Too late for the 24th's, so it goes unfired...
        assert_eq!(schedule.due(on(25, 20, 30), last_reminded), Reminder::NotDue);
        // ...and half an hour later the 25th's own reminder arrives as normal.
        assert_eq!(
            schedule.due(on(25, 21, 0), last_reminded),
            Reminder::Due {
                occurrence: on(25, 21, 0)
            }
        );
    }

    #[test]
    fn a_catch_up_an_hour_and_a_minute_short_of_the_next_still_fires() {
        let last_reminded = on(23, 21, 0);

        assert_eq!(
            schedule().due(on(25, 19, 59), last_reminded),
            Reminder::Due {
                occurrence: on(24, 21, 0)
            }
        );
    }

    #[test]
    fn a_catch_up_exactly_an_hour_short_of_the_next_is_let_go() {
        let last_reminded = on(23, 21, 0);

        assert_eq!(schedule().due(on(25, 20, 0), last_reminded), Reminder::NotDue);
    }

    #[test]
    fn a_reminder_already_sent_is_not_sent_again_that_day() {
        // The whole of "dismissing it means no more nagging today": the
        // evening wears on and Reflect stays quiet.
        let schedule = schedule();
        let sent = at(21, 0);

        assert_eq!(schedule.due(at(21, 0), sent), Reminder::NotDue);
        assert_eq!(schedule.due(at(21, 30), sent), Reminder::NotDue);
        assert_eq!(schedule.due(at(23, 59), sent), Reminder::NotDue);
    }

    #[test]
    fn a_reminder_sent_yesterday_does_not_stand_in_for_todays() {
        assert_eq!(
            schedule().due(at(21, 0), on(23, 21, 0)),
            Reminder::Due {
                occurrence: at(21, 0)
            }
        );
    }

    #[test]
    fn a_record_from_the_future_keeps_the_reminder_quiet_rather_than_firing_it() {
        // A clock moved back leaves a timestamp Reflect can't have written
        // yet. Whatever else that is, it isn't a reason to nudge.
        assert_eq!(schedule().due(at(21, 0), on(30, 21, 0)), Reminder::NotDue);
    }

    #[test]
    fn the_occurrence_before_the_hour_belongs_to_the_day_before() {
        let schedule = schedule();

        assert_eq!(schedule.occurrence_on_or_before(at(20, 59)), on(23, 21, 0));
        assert_eq!(schedule.occurrence_on_or_before(at(21, 0)), at(21, 0));
        assert_eq!(schedule.occurrence_on_or_before(at(23, 59)), at(21, 0));
    }

    #[test]
    fn a_fresh_install_starts_from_the_last_occurrence_rather_than_catching_up() {
        // Installed at ten past nine. The nine o'clock that just went by was
        // never Reflect's to deliver, so it must not fire a second later.
        let home = tempfile::tempdir().unwrap();
        let record = LastReminder::at(home.path().join("last-reminder.txt"));
        let schedule = schedule();

        let last_reminded = record.load_or_start(&schedule, at(21, 10)).unwrap();

        assert_eq!(last_reminded, at(21, 0));
        assert_eq!(schedule.due(at(21, 10), last_reminded), Reminder::NotDue);
    }

    #[test]
    fn a_reminder_sent_is_still_remembered_after_a_restart() {
        let home = tempfile::tempdir().unwrap();
        let record = LastReminder::at(home.path().join("last-reminder.txt"));

        record.record(at(21, 0)).unwrap();

        assert_eq!(
            record.load_or_start(&schedule(), at(22, 0)).unwrap(),
            at(21, 0)
        );
    }

    #[test]
    fn a_record_from_the_future_is_repaired_rather_than_left_to_expire() {
        // Left alone, a timestamp a week out would silence Reflect for a week.
        let home = tempfile::tempdir().unwrap();
        let record = LastReminder::at(home.path().join("last-reminder.txt"));
        record.record(on(31, 21, 0)).unwrap();

        assert_eq!(record.load_or_start(&schedule(), at(22, 0)).unwrap(), at(21, 0));
        // Repaired on disk too, not just for this one reading of it.
        assert_eq!(
            record.load_or_start(&schedule(), at(23, 0)).unwrap(),
            at(21, 0)
        );
    }

    #[test]
    fn a_record_nobody_can_read_starts_over_instead_of_giving_up() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("last-reminder.txt");
        std::fs::write(&path, "sometime last tuesday").unwrap();

        assert_eq!(
            LastReminder::at(&path)
                .load_or_start(&schedule(), at(22, 0))
                .unwrap(),
            at(21, 0)
        );
    }

    #[test]
    fn the_record_is_a_plain_readable_timestamp() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("last-reminder.txt");

        LastReminder::at(&path).record(at(21, 0)).unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "2026-07-24T21:00:00\n");
    }
}
