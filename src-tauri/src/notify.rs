//! The daily reminder, as a real notification from the OS.
//!
//! Reflect's one job at the appointed hour is to appear, and to open the notes
//! window when the user takes it up. The two platforms disagree about how a
//! click gets back to a running app, so each gets its own implementation
//! behind the same [`daily_reminder`] signature.
//!
//! # Why a deep link on Windows
//!
//! `ToastNotification` has an `Activated` event, and it is tempting: the tray
//! app is already running, so an in-process callback ought to be all it takes.
//! It isn't. For an unpackaged desktop app, Windows routes an activation by
//! looking for a Start Menu shortcut to the sending binary and reading
//! `System.AppUserModel.ToastActivatorCLSID` off it. Tauri's installer sets
//! `System.AppUserModel.ID` on that shortcut — which is what makes the toast
//! appear at all — but never the activator CLSID, so there is nothing for
//! Windows to activate and the event never fires.
//!
//! What does work without a COM server is protocol activation: the toast
//! carries a `launch` URI, Windows opens it like any other link, and the
//! single-instance plugin hands it to the copy of Reflect already running.
//! That is what `main.rs` listens for.
//!
//! macOS has no such problem — the click comes back in-process, as the return
//! value of the call that showed the notification.

use std::error::Error;

/// What the notification says. Deliberately not the day's writing prompt: the
/// prompt belongs on the page, and a user who has turned prompts off shouldn't
/// meet one on the lock screen anyway.
const TITLE: &str = "Reflect";
const BODY: &str = "Time to write today's reflection.";

/// The link a clicked reminder opens. Registered as a URI scheme so that
/// Windows has somewhere to send the click; see the module docs.
pub const LAUNCH_URL: &str = "reflect://write";

/// Show today's reminder.
///
/// Returns as soon as it is on screen — never blocks waiting to see what the
/// user does with it, because the caller is the scheduler thread and a
/// reminder ignored for six hours must not stop it asking again tomorrow.
///
/// `on_click` runs later, on an unspecified thread, if the user opens the
/// notification. On Windows it is never called: the click arrives as the deep
/// link described above instead, which `main.rs` routes to the same place.
pub fn daily_reminder(on_click: impl Fn() + Send + 'static) -> Result<(), Box<dyn Error>> {
    show(on_click)
}

#[cfg(windows)]
fn show(on_click: impl Fn() + Send + 'static) -> Result<(), Box<dyn Error>> {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{ToastNotification, ToastNotificationManager};

    // Windows delivers the click as a deep link, not a callback.
    let _ = on_click;

    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from(format!(
        r#"<toast activationType="protocol" launch="{launch}" duration="long">
             <visual>
               <binding template="ToastGeneric">
                 <text>{title}</text>
                 <text>{body}</text>
               </binding>
             </visual>
           </toast>"#,
        launch = LAUNCH_URL,
        title = TITLE,
        body = BODY,
    )))?;

    let toast = ToastNotification::CreateToastNotification(&xml)?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_user_model_id()))?
        .Show(&toast)?;
    Ok(())
}

/// The identity Windows files the notification under.
///
/// Release builds use the bundle identifier, which the installer writes onto
/// the Start Menu shortcut — without a shortcut carrying it, Windows drops the
/// toast silently and `Show` still reports success. A `cargo run` has no such
/// shortcut, so development borrows PowerShell's registered id purely so that
/// a toast can be seen at all; it shows up attributed to PowerShell, and is
/// not what ships.
#[cfg(windows)]
fn app_user_model_id() -> &'static str {
    if cfg!(debug_assertions) {
        r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe"
    } else {
        "com.alexfrankcodes.reflect"
    }
}

#[cfg(target_os = "macos")]
fn show(on_click: impl Fn() + Send + 'static) -> Result<(), Box<dyn Error>> {
    use mac_notification_sys::{send_notification, NotificationResponse};

    // `send_notification` doesn't return until the user has done something
    // with the notification, so it can't be called on the scheduler thread.
    // This thread exists only to wait on one notification and then end.
    std::thread::spawn(move || match send_notification(TITLE, None, BODY, None) {
        Ok(NotificationResponse::Click) => on_click(),
        // Dismissed, or left alone until it went away on its own: the day is
        // skipped, and Reflect says nothing more about it.
        Ok(_) => {}
        Err(err) => eprintln!("could not show the daily reminder: {err}"),
    });

    Ok(())
}
