//! What every one of Reflect's windows has in common.
//!
//! Each window's own module decides how big it is and what it's called; this
//! holds the choices none of them decides for itself, so that revisiting one
//! is a single edit rather than three that have to agree.

use tauri::{AppHandle, Runtime, WebviewUrl, WebviewWindowBuilder};

/// A builder for the window labelled `label`, showing `page`.
///
/// Centred and focused, because every window Reflect opens was just asked for.
/// Light rather than left to the OS: each page inside is a fixed cream canvas,
/// and a title bar that followed the OS into dark mode would sit on it as a
/// bar of unrelated colour.
pub fn builder<'a, R: Runtime>(
    app: &'a AppHandle<R>,
    label: &'a str,
    page: &str,
) -> WebviewWindowBuilder<'a, R, AppHandle<R>> {
    WebviewWindowBuilder::new(app, label, WebviewUrl::App(page.into()))
        .center()
        .focused(true)
        .theme(Some(tauri::Theme::Light))
}
