//! Whether Reflect starts itself when the user logs in.
//!
//! The preference is the user's, and lives with the others in
//! `reflect_core::settings`. This module is only the glue that makes the OS
//! agree with it — registering and unregistering, and saying so when it can't.
//!
//! Windows only. The plugin underneath handles macOS as well, but this release
//! is Windows alone and nobody has ever run the macOS build; writing a
//! LaunchAgent from a binary that has never been started is not a change worth
//! making unseen. Everywhere else [`SUPPORTED`] is false, [`apply`] does
//! nothing, and the Settings window leaves the row out rather than offering a
//! switch that moves nothing.

use std::error::Error;

use tauri::{AppHandle, Runtime};

/// Whether this platform starts Reflect at login at all.
///
/// The Settings window asks before drawing its third row: a switch that
/// silently does nothing is worse than no switch.
pub const SUPPORTED: bool = cfg!(windows);

/// Make the OS agree with `wanted`.
///
/// Idempotent, because it is called on every startup as well as on every
/// change — a machine where the registration was removed behind Reflect's back
/// gets it put right at the next launch, and one where it is already correct
/// is left alone.
#[cfg(windows)]
pub fn apply<R: Runtime>(app: &AppHandle<R>, wanted: bool) -> Result<(), Box<dyn Error>> {
    use tauri_plugin_autostart::ManagerExt;

    let manager = app.autolaunch();
    // Asked rather than assumed: `enable` on an already-registered app rewrites
    // the registry entry for no reason, and this runs at every startup.
    if manager.is_enabled()? == wanted {
        return Ok(());
    }

    if wanted {
        manager.enable()?;
    } else {
        manager.disable()?;
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn apply<R: Runtime>(app: &AppHandle<R>, wanted: bool) -> Result<(), Box<dyn Error>> {
    let _ = (app, wanted);
    Ok(())
}

/// [`apply`], for callers with nowhere useful to report a failure to.
///
/// A registration that didn't take is worth saying out loud but is never worth
/// stopping for: Reflect running now matters more than Reflect running after
/// the next login.
pub fn apply_or_report<R: Runtime>(app: &AppHandle<R>, wanted: bool) {
    if let Err(err) = apply(app, wanted) {
        eprintln!("could not set whether Reflect starts at login: {err}");
    }
}
