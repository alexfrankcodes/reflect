//! The Browse window, and the two calls the page inside it makes.
//!
//! Which days there are, and what was written on one, both come from
//! `reflect_core::entries`. This module opens a window and passes the answers
//! through — there is nothing here to decide, because browsing changes nothing.

use reflect_core::entries::{format_entry_date, parse_entry_date, Entries};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// The one browse window there ever is.
const WINDOW_LABEL: &str = "browse";

/// Told to the browse window when the days it is listing are no longer the
/// days on disk. The page answers by drawing the list again, keeping whatever
/// day it was showing.
const ENTRIES_CHANGED: &str = "browse-again";

/// Tell a browse window, if one is open, that the entries have moved on.
///
/// A window left open while the user writes would otherwise go on showing the
/// journal as it stood when they opened it.
pub fn entries_changed<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        if let Err(err) = window.emit_to(WINDOW_LABEL, ENTRIES_CHANGED, ()) {
            eprintln!("could not refresh the browse window: {err}");
        }
    }
}

/// [`open`], for the tray menu, which has nowhere useful to report a failure to.
pub fn open_or_report<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = open(app) {
        eprintln!("could not open the browse window: {err}");
    }
}

/// Open the browse window, or bring it forward if it's already open.
pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        // Reflect isn't the only thing that can write in the entries folder —
        // "Reveal Entries Folder" invites the user into it. Asking the tray
        // for Browse is asking for the entries as they stand.
        entries_changed(app);
        window.set_focus()?;
        return Ok(());
    }

    crate::window::builder(app, WINDOW_LABEL, "browse.html")
        .title("Entries")
        // Wider than the notes window and no taller: two columns rather than
        // one, and the same measure of text to read in the right-hand one.
        .inner_size(720.0, 460.0)
        .min_inner_size(520.0, 300.0)
        .build()?;

    Ok(())
}

/// Every day there is something to read, most recent first, as `YYYY-MM-DD`.
///
/// Dates rather than sentences: how a date is spelled belongs to the machine
/// this is running on, so the page writes it out in the reader's own locale.
#[tauri::command]
pub fn browse_dates(entries: State<'_, Entries>) -> Result<Vec<String>, String> {
    Ok(entries
        .dates()
        .map_err(|err| format!("couldn't read your entries: {err}"))?
        .into_iter()
        .map(format_entry_date)
        .collect())
}

/// What was written on `date`, exactly as it was saved.
#[tauri::command]
pub fn browse_entry(entries: State<'_, Entries>, date: String) -> Result<String, String> {
    let day = parse_entry_date(&date)
        .ok_or_else(|| format!("{date:?} isn't a date Reflect understands."))?;

    // A day with nothing on it is a day with no file, so neither of these is
    // reachable by anything Reflect wrote — a day can only disappear between
    // the list being drawn and it being clicked, and only a hand can leave an
    // empty file behind. Both are said rather than shown, because a blank
    // reading pane under a full date reads as a bug and this doesn't.
    entries
        .load(day)
        .map_err(|err| format!("couldn't read that entry: {err}"))?
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| "There's nothing written on that day.".to_owned())
}
