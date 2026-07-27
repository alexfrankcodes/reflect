//! Pure business logic for Reflect.
//!
//! Nothing in this crate may depend on Tauri, a webview, or the OS event loop.
//! The Tauri layer in `src-tauri` is a thin adapter over what lives here, so
//! this crate's behaviour stays testable with a plain `cargo test`.

pub mod entries;
pub mod notes;
pub mod prompts;
pub mod schedule;
pub mod tray_menu;

/// A calendar date, for tests that need one to talk about.
#[cfg(test)]
fn day(year: i32, month: u32, day: u32) -> chrono::NaiveDate {
    chrono::NaiveDate::from_ymd_opt(year, month, day).expect("test date must be a real date")
}
