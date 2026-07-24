//! Pure business logic for Reflect.
//!
//! Nothing in this crate may depend on Tauri, a webview, or the OS event loop.
//! The Tauri layer in `src-tauri` is a thin adapter over what lives here, so
//! this crate's behaviour stays testable with a plain `cargo test`.

pub mod tray_menu;
