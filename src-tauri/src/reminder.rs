//! The thread that watches the clock.
//!
//! Reflect keeps no timer set for the appointed hour. It asks, over and over,
//! whether a reminder is owed — and that is the whole of how a machine asleep
//! at nine still gets nudged: it wakes, the next tick asks, and the answer is
//! yes. The rule doing the answering is [`Schedule`], in the core, where it is
//! tested against a clock that can be told what time it is.

use std::time::Duration;

use chrono::Local;
use reflect_core::schedule::{Reminder, Schedule};
use tauri::{AppHandle, Manager, Runtime};

use crate::preferences::Preferences;

/// How often the question gets asked. Frequent enough that the reminder lands
/// within a minute of the time the user set, and cheap enough that asking all
/// day costs nothing worth measuring.
const TICK: Duration = Duration::from_secs(30);

/// Start watching for the daily reminder. Runs until the app exits.
///
/// Requires [`Preferences`] to have been managed already — it holds both the
/// settings this reads and the record it writes.
pub fn start<R: Runtime>(app: &AppHandle<R>) {
    let app = app.clone();
    // Read from the config rather than written out again here: whose
    // notification this is, and the identity the installer registers, have to
    // be the same string.
    let app_id = app.config().identifier.clone();

    std::thread::spawn(move || loop {
        if let Err(err) = ask(&app, &app_id) {
            // A tick that fails is not a reason to stop asking — the disk
            // being busy this second says nothing about the next one.
            eprintln!("could not check whether a reminder is due: {err}");
        }
        std::thread::sleep(TICK);
    });
}

fn ask<R: Runtime>(app: &AppHandle<R>, app_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let preferences = app.state::<Preferences>();
    // Held across the whole of read-decide-record. The Settings window is the
    // other writer of this record, and a change to the daily time arriving
    // halfway through this would be answered with the time it replaced.
    let files = preferences.lock();

    // Both read from disk each time rather than held in memory: a record
    // repaired on disk stays repaired, and a daily time the user changed a
    // minute ago is the one this tick asks about.
    let now = Local::now().naive_local();
    let schedule = Schedule::daily_at(files.settings.load()?.daily_time);
    let last_reminded = files.last_reminder.load_or_start(&schedule, now)?;

    let Reminder::Due { occurrence } = schedule.due(now, last_reminded) else {
        return Ok(());
    };

    // Handed to the OS first, recorded second, so that a reminder Reflect
    // never managed to hand over is still owed on the next tick. Only that
    // much is guaranteed: either platform can accept a notification and then
    // decline to show it — a notification setting, a focus session — and
    // neither tells us. What is recorded is that Reflect asked for the nudge,
    // which is the most it ever knows.
    let app = app.clone();
    crate::notify::daily_reminder(app_id, move || {
        // Onto the main thread before building a window. macOS hands the click
        // back on a thread of its own, and AppKit refuses to make a window
        // anywhere but the main one — quietly, which is the worst way to find
        // out. Windows arrives here already on the main thread and is unharmed.
        let opening = app.clone();
        if let Err(err) = app.run_on_main_thread(move || crate::notes::open_or_report(&opening)) {
            eprintln!("could not reach the main thread to open the notes window: {err}");
        }
    })?;

    // The occurrence, not the moment it appeared: a catch-up shown on Tuesday
    // morning is still Monday evening's reminder.
    files.last_reminder.record(occurrence)?;
    Ok(())
}
