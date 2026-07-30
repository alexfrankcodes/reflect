//! The Settings window, and the two calls the page inside it makes.
//!
//! The decisions — what a valid time looks like, what a missing setting falls
//! back to, what a changed time means for a reminder not yet delivered — all
//! live in `reflect_core::settings` and `reflect_core::schedule`. This module
//! opens a window, reads the file, and writes it back.

use chrono::Local;
use reflect_core::settings::{format_daily_time, parse_daily_time, Settings};
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State};

use crate::preferences::Preferences;

/// The one settings window there ever is.
const WINDOW_LABEL: &str = "settings";

/// What the settings page shows when it opens.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    /// `HH:MM`, which is what `<input type="time">` wants and what the
    /// settings file already holds.
    daily_time: String,
    show_prompts: bool,
    /// `None` where the platform doesn't start apps at login, which is how the
    /// page knows to leave the row out altogether rather than draw a switch
    /// that moves nothing. See `autostart.rs`.
    start_at_login: Option<bool>,
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

    crate::window::builder(app, WINDOW_LABEL, "settings.html")
        .title("Settings")
        // The rows and a line of text; there is nothing here that reflows, so
        // a resize would only ever spread it thinner. The height is the
        // content's, with room left under the rows for the line at the foot to
        // sit apart from what it describes — and one row shorter where the
        // start-at-login row isn't drawn, rather than leaving a gap where it
        // would have been.
        .inner_size(
            400.0,
            if crate::autostart::SUPPORTED {
                218.0
            } else {
                178.0
            },
        )
        .resizable(false)
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

    Ok(page(&settings))
}

/// Put `daily_time`, `show_prompts` and `start_at_login` into force.
///
/// Returning an error leaves the settings as they were and says so on the page,
/// which is the only honest outcome for a change that didn't take: a Settings
/// window showing a time Reflect isn't using is worse than no Settings window.
/// That is why the reminder record is corrected first and the settings written
/// second — the other order can fail with the new time already in force and its
/// record uncorrected, which is the one arrangement that produces a nudge the
/// moment the user closes Settings. A correction that lands and a write that
/// then fails costs at most a single day's nudge, and this codebase already
/// holds that a missed reminder beats two almost back to back.
///
/// The OS registration goes first of the three, for the same reason read the
/// other way: a refusal from Windows then leaves the record, the file and the
/// registry all as they were, and there is nothing to unpick. Should either of
/// the writes after it fail, the next startup reconciles the registry to
/// whatever the file says — unless the file can't be read then either, in which
/// case Reflect leaves the disagreement alone rather than guess which side of
/// it the user meant.
///
/// `start_at_login` is ignored where the platform doesn't do it, because the
/// page never offered the row and its checkbox is no one's answer.
#[tauri::command]
pub fn settings_save(
    app: AppHandle,
    preferences: State<'_, Preferences>,
    daily_time: String,
    show_prompts: bool,
    start_at_login: bool,
) -> Result<Page, String> {
    // Refused rather than quietly rounded to a default — standing 9pm in place
    // of something Reflect couldn't read is how a user's chosen time changes
    // without anything telling them.
    let daily_time = parse_daily_time(&daily_time)
        .ok_or_else(|| format!("{daily_time:?} isn't a time of day Reflect understands."))?;

    let files = preferences.lock();
    let in_force = files
        .settings
        .load()
        .map_err(|err| format!("couldn't read your settings: {err}"))?;

    let chosen = Settings {
        daily_time,
        show_prompts,
        start_at_login: if crate::autostart::SUPPORTED {
            start_at_login
        } else {
            in_force.start_at_login
        },
    };

    crate::autostart::apply(&app, chosen.start_at_login)
        .map_err(|err| format!("couldn't set whether Reflect starts at login: {err}"))?;

    // A time that hasn't moved leaves the record alone, which is the whole of
    // why both times are handed over rather than only the new one.
    files
        .last_reminder
        .follow_time_change(
            in_force.daily_time,
            chosen.daily_time,
            Local::now().naive_local(),
        )
        .map_err(|err| format!("couldn't reschedule your reminder: {err}"))?;

    files
        .settings
        .save(&chosen)
        .map_err(|err| format!("couldn't save your settings: {err}"))?;

    // Handed back rather than assumed: the page then shows what Reflect is
    // actually going to do, spelled the way Reflect spells it.
    Ok(page(&chosen))
}

fn page(settings: &Settings) -> Page {
    Page {
        daily_time: format_daily_time(settings.daily_time),
        show_prompts: settings.show_prompts,
        start_at_login: crate::autostart::SUPPORTED.then_some(settings.start_at_login),
    }
}
