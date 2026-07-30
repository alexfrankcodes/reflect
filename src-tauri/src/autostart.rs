//! Whether Reflect starts itself when the user logs in.
//!
//! The preference is the user's, and lives with the others in
//! `reflect_core::settings`. This module is only the glue that makes the OS
//! agree with it — registering and unregistering, and saying so when it can't.
//!
//! Windows only, following ADR 0002: the plugin underneath handles macOS as
//! well, but that build has never been run by anyone and writing a LaunchAgent
//! from a binary nobody has started is not a change worth making unseen.
//! Everywhere else [`SUPPORTED`] is false, [`apply`] does nothing, and the
//! Settings window leaves the row out rather than offering a switch that moves
//! nothing.
//!
//! # Why the registry is read directly
//!
//! The plugin offers `is_enabled`, and it is the obvious thing to ask before
//! changing anything. It answers a different question than it appears to.
//! It reports whether a value of Reflect's name exists under `Run` — never
//! whether that value still points at *this* binary — and then ands in
//! Windows' own Task Manager startup override. Two things follow, and both
//! are the reason this module asks the registry itself.
//!
//! An entry left by an older install, or by a build in another folder, reads
//! as "already enabled", so a check that trusted it would leave the stale path
//! in place forever. And an entry the user switched off in Task Manager reads
//! as "not enabled" while the value is still sitting there, so turning this
//! off in Settings would delete nothing and the entry would come back to life
//! the moment Task Manager was switched back.

use std::error::Error;

use tauri::{AppHandle, Runtime};

/// Whether this platform starts Reflect at login at all.
///
/// The Settings window asks before drawing its third row: a switch that
/// silently does nothing is worse than no switch.
pub const SUPPORTED: bool = cfg!(windows);

/// Where Windows keeps the programs this user starts at login, and the key the
/// plugin underneath writes into.
#[cfg(windows)]
const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Make the OS agree with `wanted`.
///
/// Idempotent, because it is called at every startup as well as on every
/// change — a machine where the entry was removed behind Reflect's back gets it
/// put back at the next launch, and one where it points at a binary that has
/// since moved gets it corrected.
#[cfg(windows)]
pub fn apply<R: Runtime>(app: &AppHandle<R>, wanted: bool) -> Result<(), Box<dyn Error>> {
    use tauri_plugin_autostart::ManagerExt;

    if wanted {
        // Written every time rather than only when nothing is there. The entry
        // records the path of the binary that wrote it, so an upgrade that
        // moves Reflect leaves the old one pointing at a file that is gone —
        // and rewriting a registry value that already says the right thing
        // costs nothing worth saving.
        app.autolaunch().enable()?;
        return Ok(());
    }

    // Deleting a value that isn't there is an error, and no entry is the
    // ordinary state at every startup once the user has turned this off. So
    // ask first — of the registry, for the reason in the module docs.
    if registered(app)? {
        app.autolaunch().disable()?;
    }
    Ok(())
}

/// Whether Windows holds a startup entry for Reflect at all, whatever it points
/// at and whether or not Task Manager has it switched off.
#[cfg(windows)]
fn registered<R: Runtime>(app: &AppHandle<R>) -> Result<bool, Box<dyn Error>> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    // The same name the plugin files the entry under: the product name, which
    // is what `package_info` carries.
    let name = &app.package_info().name;
    let run = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_READ)?;
    Ok(run.get_value::<String, _>(name).is_ok())
}

/// Nothing to do: this platform doesn't start Reflect at login, and the
/// Settings window doesn't offer to. See the module docs.
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
