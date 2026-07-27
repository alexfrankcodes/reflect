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
//! Something may still be owed to the installer, though — unconfirmed, and
//! worth checking against Microsoft's own documentation before anyone relies
//! on it. A stub `ToastActivatorCLSID` on that same shortcut — any GUID, with
//! no COM server behind it — *appears* to be what makes Windows keep a
//! notification in the Action Centre once its banner has gone. If that reading
//! is right, the reminder is currently live only while it is on screen, and
//! someone away from their desk at nine can't come back and click it. Reflect's
//! nudge is one quiet moment in the day, so that would be worth fixing when
//! packaging is taken up; it needs a shell-link property the bundler doesn't
//! write today, not a change in this file.
//!
//! # Why the old API on macOS
//!
//! The spec named `UNUserNotificationCenter`, which is indeed the modern way.
//! It also refuses to deliver anything from a bundle that isn't code-signed,
//! which would make the daily reminder — the whole point of Reflect — depend
//! on a paid developer certificate before it would work at all, including for
//! anyone who cloned the repo and built it themselves. `NSUserNotification`
//! underneath `mac-notification-sys` is deprecated and asks for none of that,
//! and hands the click straight back in-process. Worth revisiting if signing
//! becomes part of the release anyway.

use std::error::Error;

use reflect_core::notification::{REMINDER_BODY, REMINDER_TITLE};

/// The link a clicked reminder opens. Registered as a URI scheme so that
/// Windows has somewhere to send the click; see the module docs.
#[cfg_attr(not(windows), allow(dead_code))]
const LAUNCH_URL: &str = "reflect://write";

/// Show today's reminder.
///
/// Returns as soon as the OS has taken it — never blocks waiting to see what
/// the user does with it, because the caller is the scheduler thread and a
/// reminder ignored for six hours must not stop it asking again tomorrow.
///
/// `app_id` is the bundle identifier from `tauri.conf.json`. Both platforms
/// need it to say whose notification this is; neither will use Reflect's own
/// name without it.
///
/// `on_click` runs later, on an unspecified thread, if the user opens the
/// notification. On Windows it is never called: the click arrives as the deep
/// link described above instead, which `main.rs` routes to the same place.
pub fn daily_reminder(
    app_id: &str,
    on_click: impl Fn() + Send + 'static,
) -> Result<(), Box<dyn Error>> {
    show(app_id, on_click)
}

#[cfg(windows)]
fn show(app_id: &str, on_click: impl Fn() + Send + 'static) -> Result<(), Box<dyn Error>> {
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
        title = REMINDER_TITLE,
        body = REMINDER_BODY,
    )))?;

    let toast = ToastNotification::CreateToastNotification(&xml)?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_user_model_id(app_id)))?
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
fn app_user_model_id(app_id: &str) -> String {
    if cfg!(debug_assertions) {
        r"{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\WindowsPowerShell\v1.0\powershell.exe".to_owned()
    } else {
        app_id.to_owned()
    }
}

#[cfg(target_os = "macos")]
fn show(app_id: &str, on_click: impl Fn() + Send + 'static) -> Result<(), Box<dyn Error>> {
    use mac_notification_sys::{send_notification, set_application, NotificationResponse};

    // Without this the notification goes out under whatever bundle the crate
    // falls back to — Finder's — so it would arrive wearing the wrong name and
    // obey Finder's notification settings rather than Reflect's. It sticks for
    // the life of the process and refuses a second call, which is why a repeat
    // isn't treated as a failure.
    let _ = set_application(app_id);

    // `send_notification` doesn't return until the user has done something
    // with the notification, so it can't be called on the scheduler thread.
    // This thread exists only to wait on one notification and then end.
    std::thread::spawn(move || {
        match send_notification(REMINDER_TITLE, None, REMINDER_BODY, None) {
            Ok(NotificationResponse::Click) => on_click(),
            // Dismissed, or left alone until the banner slid away: the day is
            // skipped and Reflect says nothing more about it. A banner that times
            // out reports the same as one dismissed, so opening it from
            // Notification Center later isn't heard — the tray is the way back in.
            Ok(_) => {}
            Err(err) => eprintln!("could not show the daily reminder: {err}"),
        }
    });

    Ok(())
}
