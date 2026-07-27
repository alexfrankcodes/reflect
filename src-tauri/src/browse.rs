//! The Browse window, and the two calls the page inside it makes.
//!
//! Which days there are, and what was written on one, both come from
//! `reflect_core::entries`. This module opens a window and passes the answers
//! through — there is nothing here to decide, because browsing changes nothing.

use reflect_core::entries::{format_entry_date, parse_entry_date, Entries};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder};

/// The one browse window there ever is.
const WINDOW_LABEL: &str = "browse";

/// [`open`], for the tray menu, which has nowhere useful to report a failure to.
pub fn open_or_report<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = open(app) {
        eprintln!("could not open the browse window: {err}");
    }
}

/// Open the browse window, or bring it forward if it's already open.
pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        // A window left open since before today's writing would otherwise come
        // forward still showing a list that today isn't on. Asking for the
        // tray is asking for the entries as they stand.
        window.emit_to(WINDOW_LABEL, "browse-again", ())?;
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("browse.html".into()))
        .title("Entries")
        // Wider than the notes window and no taller: two columns rather than
        // one, and the same measure of text to read in the right-hand one.
        .inner_size(720.0, 460.0)
        .min_inner_size(520.0, 300.0)
        .center()
        .focused(true)
        // Light for the same reason the other two windows are: the page is a
        // fixed cream canvas, and a dark title bar would sit on it as a bar of
        // unrelated colour.
        .theme(Some(tauri::Theme::Light))
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

    // Only ever reachable by a day disappearing between the list being drawn
    // and it being clicked — but a blank page reads as a bug, and this doesn't.
    entries
        .load(day)
        .map_err(|err| format!("couldn't read that entry: {err}"))?
        .ok_or_else(|| "There's nothing written on that day.".to_owned())
}
