//! The notes window, and the two calls the page inside it makes.
//!
//! The decisions — which prompt, what counts as having written something,
//! whether that means touching the file — all live in `reflect_core::notes`.
//! This module opens a window, hands the page its text, and hands the text
//! back when the window closes.

use std::path::PathBuf;
use std::sync::Mutex;

use chrono::Local;
use reflect_core::entries::Entries;
use reflect_core::notes::NotesSession;
use serde::Serialize;
use tauri::{AppHandle, Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder};

use crate::preferences::Preferences;

/// The one notes window there ever is.
const WINDOW_LABEL: &str = "notes";

/// Reflect's writing state: where entries live, and the day currently open.
pub struct Notes {
    entries: Entries,
    /// `Some` from the moment the page asks for its text until the window
    /// closes. A window that closes without ever having asked has nothing on
    /// it to save.
    open_session: Mutex<Option<NotesSession>>,
}

impl Notes {
    pub fn with_entries_in(dir: PathBuf) -> Self {
        Self {
            entries: Entries::in_dir(dir),
            open_session: Mutex::new(None),
        }
    }
}

/// What the notes page shows when it opens.
#[derive(Serialize)]
pub struct Page {
    /// `None` where the user has turned prompts off — the page then draws no
    /// prompt line at all rather than an empty one.
    prompt: Option<String>,
    text: String,
}

/// [`open`], for the callers who have nowhere useful to report a failure to —
/// a clicked notification, a tray menu item, a second launch. There is nothing
/// to tell the user that a window they asked for didn't appear, beyond the
/// window not appearing.
pub fn open_or_report<R: Runtime>(app: &AppHandle<R>) {
    if let Err(err) = open(app) {
        eprintln!("could not open the notes window: {err}");
    }
}

/// Open the notes window on today's page, or bring it forward if it's already
/// open — a second click shouldn't mean a second window over the same day.
pub fn open<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(WINDOW_LABEL) {
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(app, WINDOW_LABEL, WebviewUrl::App("notes.html".into()))
        .title("Reflect")
        .inner_size(480.0, 380.0)
        .min_inner_size(320.0, 220.0)
        .center()
        .focused(true)
        // The page inside is a fixed cream canvas, so the title bar is asked
        // to be light too rather than left to follow the OS into dark mode and
        // sit on the window like a bar of unrelated colour.
        .theme(Some(tauri::Theme::Light))
        .build()?;

    Ok(())
}

/// Quit, but not out from under someone mid-sentence: if the notes window is
/// open, ask it to close first — that runs the same save the close button does
/// — and take the app down once it has gone.
pub fn quit<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(WINDOW_LABEL) else {
        app.exit(0);
        return Ok(());
    };

    // The listener outlives a close the page refuses (a failed save keeps the
    // window open and says so in the prompt line). That is deliberate: quit
    // was asked for, so the app goes down as soon as the day's writing is
    // safely on disk, whenever the user manages that.
    let app = app.clone();
    window.on_window_event(move |event| {
        if let tauri::WindowEvent::Destroyed = event {
            app.exit(0);
        }
    });

    // Surfaced before it's asked to close, so that a window refusing to go —
    // it having failed to save — is in front of the user rather than buried
    // behind whatever they were doing when they hit Quit.
    window.set_focus()?;
    window.close()?;

    Ok(())
}

/// Today's page: its prompt, and whatever is already written on it.
#[tauri::command]
pub fn notes_page(
    notes: State<'_, Notes>,
    preferences: State<'_, Preferences>,
) -> Result<Page, String> {
    // Read at open rather than held from startup, so that turning prompts off
    // in Settings shows on the very next page rather than the next launch.
    let settings = preferences
        .lock()
        .settings
        .load()
        .map_err(|err| format!("couldn't read your settings: {err}"))?;

    // Today is settled here rather than when the window was built, so a window
    // opened seconds before midnight belongs to the day its page was drawn for
    // — the same day the entry will be filed under when it closes.
    let session = NotesSession::open(&notes.entries, &settings, Local::now().date_naive())
        .map_err(|err| format!("couldn't read today's entry: {err}"))?;

    let page = Page {
        prompt: session.prompt().map(str::to_owned),
        text: session.opened_with().to_owned(),
    };
    *notes.open_session.lock().expect("notes session lock") = Some(session);

    Ok(page)
}

/// The window is closing on `text`. Returning an error keeps it open, so
/// nothing the user wrote disappears behind a failed write.
#[tauri::command]
pub fn notes_close(notes: State<'_, Notes>, text: String) -> Result<(), String> {
    let session = notes
        .open_session
        .lock()
        .expect("notes session lock")
        .take();

    let Some(session) = session else {
        return Ok(());
    };

    session
        .close(&notes.entries, &text)
        .map(|_| ())
        .map_err(|err| {
            // Put it back: the window stays open on the failure, and closing it
            // again should try the same save again rather than lose the day.
            *notes.open_session.lock().expect("notes session lock") = Some(session);
            format!("couldn't save today's entry: {err}")
        })
}
