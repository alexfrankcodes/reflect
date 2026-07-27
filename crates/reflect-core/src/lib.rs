//! Pure business logic for Reflect.
//!
//! Nothing in this crate may depend on Tauri, a webview, or the OS event loop.
//! The Tauri layer in `src-tauri` is a thin adapter over what lives here, so
//! this crate's behaviour stays testable with a plain `cargo test`.

pub mod entries;
pub mod notes;
pub mod notification;
pub mod prompts;
pub mod schedule;
pub mod settings;
pub mod tray_menu;

/// Read a file Reflect may simply not have written yet.
///
/// Reflect's files are all optional in the same way — a day nobody wrote on,
/// a reminder that has never gone out — so a missing one is an answer of
/// `None`, never an error to handle.
fn read_if_written(path: &std::path::Path) -> std::io::Result<Option<String>> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

/// A calendar date, for tests that need one to talk about.
#[cfg(test)]
fn day(year: i32, month: u32, day: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(year, month, day).expect("test date must be a real date")
}
