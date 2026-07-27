//! The thread that watches the clock.
//!
//! Reflect keeps no timer set for the appointed hour. It asks, over and over,
//! whether a reminder is owed — and that is the whole of how a machine asleep
//! at nine still gets nudged: it wakes, the next tick asks, and the answer is
//! yes. The rule doing the answering is [`Schedule`], in the core, where it is
//! tested against a clock that can be told what time it is.

use std::path::PathBuf;
use std::time::Duration;

use chrono::Local;
use reflect_core::schedule::{LastReminder, Reminder, Schedule, DEFAULT_DAILY_TIME};
use tauri::{AppHandle, Runtime};

/// How often the question gets asked. Frequent enough that the reminder lands
/// within a minute of the time the user set, and cheap enough that asking all
/// day costs nothing worth measuring.
const TICK: Duration = Duration::from_secs(30);

/// Start watching for the daily reminder. Runs until the app exits.
pub fn start<R: Runtime>(app: &AppHandle<R>, record_path: PathBuf) {
    let app = app.clone();

    std::thread::spawn(move || {
        let schedule = Schedule::daily_at(DEFAULT_DAILY_TIME);
        let record = LastReminder::at(record_path);

        loop {
            if let Err(err) = ask(&app, &schedule, &record) {
                // A tick that fails is not a reason to stop asking — the disk
                // being busy this second says nothing about the next one.
                eprintln!("could not check whether a reminder is due: {err}");
            }
            std::thread::sleep(TICK);
        }
    });
}

fn ask<R: Runtime>(
    app: &AppHandle<R>,
    schedule: &Schedule,
    record: &LastReminder,
) -> Result<(), Box<dyn std::error::Error>> {
    // Read from disk each time rather than held in memory, so that a record
    // repaired on disk stays repaired, and so #16 can move the time under a
    // running app without restarting it.
    let now = Local::now().naive_local();
    let last_reminded = record.load_or_start(schedule, now)?;

    let Reminder::Due { occurrence } = schedule.due(now, last_reminded) else {
        return Ok(());
    };

    // Shown first, recorded second. A notification that failed to appear is
    // owed again on the next tick; recorded first, it would be silently marked
    // delivered and the day lost.
    let app = app.clone();
    crate::notify::daily_reminder(move || {
        if let Err(err) = crate::notes::open(&app) {
            eprintln!("could not open the notes window: {err}");
        }
    })?;

    // The occurrence, not the moment it appeared: a catch-up shown on Tuesday
    // morning is still Monday evening's reminder.
    record.record(occurrence)?;
    Ok(())
}
