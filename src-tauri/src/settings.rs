//! The Settings window, and the two calls the page inside it makes.
//!
//! The decisions — what a valid time looks like, what a missing setting falls
//! back to, what a changed time means for a reminder not yet delivered — all
//! live in `reflect_core::settings` and `reflect_core::schedule`. This module
//! opens a window, reads the file, and writes it back.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use chrono::Local;
use reflect_core::schedule::{LastReminder, Schedule};
use reflect_core::settings::{parse_daily_time, Settings, SettingsFile};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder};

/// The one settings window there ever is.
const WINDOW_LABEL: &str = "settings";

/// The two files that between them decide when Reflect nudges — the settings
/// the user set, and the record of the last nudge — behind a single lock.
///
/// Locked together because they are read and written together, by two threads
/// that disagree about the answer. The reminder thread reads the settings,
/// decides whether a nudge is owed, and records it; the Settings window writes
/// the settings and then corrects the record for the new time. Interleave those
/// two and a tick still holding the old time can fire the very reminder the
/// user's change was meant to move — and then record over the correction, so
/// that changing the time is what produces a notification a second later.
pub struct Preferences {
    files: Mutex<Files>,
}

/// The settings and the reminder record, only reachable together. See
/// [`Preferences`].
pub struct Files {
    pub settings: SettingsFile,
    pub last_reminder: LastReminder,
}

impl Preferences {
    pub fn new(settings_path: PathBuf, last_reminder_path: PathBuf) -> Self {
        Self {
            files: Mutex::new(Files {
                settings: SettingsFile::at(settings_path),
                last_reminder: LastReminder::at(last_reminder_path),
            }),
        }
    }

    /// Both files, held exclusively until the guard is dropped. Hold it across
    /// the whole of a read-decide-write, not just the read.
    pub fn lock(&self) -> MutexGuard<'_, Files> {
        self.files.lock().expect("preferences lock")
    }
}

/// What the settings page shows when it opens.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// `HH:MM`, which is what `<input type="time">` wants and what the
    /// settings file already holds.
    daily_time: String,
    show_prompts: bool,
}

/// [`open`], for the tray menu, which has nowhere useful to report a failure to.
pub fn open_or_report<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = open(app) {
        eprintln!("could not open the settings window: {err}");
    }
}

/// Open the settings window, or bring it forward if it's already open.
pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("settings.html".into()))
        .title("Settings")
        // Two rows and a line of text; there is nothing here that reflows, so
        // a resize would only ever spread it thinner. The height is the
        // content's, with room left under the rows for the line at the foot to
        // sit apart from what it describes.
        .inner_size(400.0, 178.0)
        .resizable(false)
        .center()
        .focused(true)
        // Light for the same reason the notes window is: the page is a fixed
        // cream canvas, and a dark title bar would sit on it as a bar of
        // unrelated colour.
        .theme(Some(tauri::Theme::Light))
        .build()?;

    Ok(())
}

/// The settings as they stand.
#[tauri::command]
pub fn settings_page(preferences: State<'_, Preferences>) -> Result<Page, String> {
    let settings = preferences
        .lock()
        .settings
        .load()
        .map_err(|err| format!("couldn't read your settings: {err}"))?;

    Ok(Page {
        daily_time: settings.daily_time.format("%H:%M").to_string(),
        show_prompts: settings.show_prompts,
    })
}

/// Put `daily_time` and `show_prompts` into force.
///
/// Returning an error leaves the settings as they were and says so on the page,
/// which is the only honest outcome for a change that didn't take: a Settings
/// window showing a time Reflect isn't using is worse than no Settings window.
#[tauri::command]
pub fn settings_save(
    preferences: State<'_, Preferences>,
    daily_time: String,
    show_prompts: bool,
) -> Result<Page, String> {
    // Refused rather than quietly rounded to a default — standing 9pm in place
    // of something Reflect couldn't read is how a user's chosen time changes
    // without anything telling them.
    let daily_time = parse_daily_time(&daily_time)
        .ok_or_else(|| format!("{daily_time:?} isn't a time of day Reflect understands."))?;
    let chosen = Settings {
        daily_time,
        show_prompts,
    };

    let files = preferences.lock();
    let in_force = files
        .settings
        .load()
        .map_err(|err| format!("couldn't read your settings: {err}"))?;

    files
        .settings
        .save(&chosen)
        .map_err(|err| format!("couldn't save your settings: {err}"))?;

    // Only on a time that actually moved. Run on every save, this would swallow
    // a reminder still owed from a machine that was asleep at the hour — so
    // merely toggling prompts would cost the user that day's nudge.
    if chosen.daily_time != in_force.daily_time {
        files
            .last_reminder
            .skip_past_occurrences(
                &Schedule::daily_at(chosen.daily_time),
                Local::now().naive_local(),
            )
            .map_err(|err| format!("couldn't reschedule your reminder: {err}"))?;
    }

    // Handed back rather than assumed: the page then shows what Reflect is
    // actually going to do, spelled the way Reflect spells it.
    Ok(Page {
        daily_time: chosen.daily_time.format("%H:%M").to_string(),
        show_prompts: chosen.show_prompts,
    })
}
