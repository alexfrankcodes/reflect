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
//! It isn't free, though. In-process activation for an unpackaged app means
//! registering a real `INotificationActivationCallback` COM server and
//! pointing a Start Menu shortcut's `System.AppUserModel.ToastActivatorCLSID`
//! at it — a COM server, a shortcut the app has to write and maintain, and an
//! uninstaller that has to clean up after both, for one notification a day.
//!
//! Protocol activation asks for none of that: the toast carries a `launch`
//! URI, Windows opens it like any other link, and the single-instance plugin
//! hands it to the copy of Reflect already running. That is what `main.rs`
//! listens for, and it is measured to work from the notification centre just
//! as it does from the banner.
//!
//! Nothing here needs a Start Menu shortcut. An earlier reading of Microsoft's
//! documentation had it that a reminder is discarded once its banner fades
//! unless a shortcut carries a stub activator CLSID; that was measured and it
//! is false, along with the belief that a toast is dropped outright when no
//! shortcut carries its identity. A shortcut supplies the name and icon the
//! reminder appears under and nothing else — which is why a Reflect that was
//! built rather than installed shows its bare identifier. ADR 0001 records
//! what was run.
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

    // The identity Windows files the notification under, the same in every
    // build. It does not have to resolve to a Start Menu shortcut for the
    // reminder to be shown, kept, or clicked; where a shortcut carrying it
    // exists — the one the installer places — the reminder wears Reflect's
    // name and icon instead of this string.
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn show(app_id: &str, on_click: impl Fn() + Send + 'static) -> Result<(), Box<dyn Error>> {
    use mac_notification_sys::{
        send_notification, set_application, Notification, NotificationResponse,
    };

    // Without this the notification goes out under whatever bundle the crate
    // falls back to — Finder's — so it would arrive wearing the wrong name and
    // obey Finder's notification settings rather than Reflect's. It sticks for
    // the life of the process and refuses a second call, which is why a repeat
    // isn't treated as a failure.
    let _ = set_application(app_id);

    // Asking to be told about the click is what makes the click reachable at
    // all. Left off, the call hands the banner to the OS and reports "nothing
    // happened" in the same breath, forgetting the notification as it goes —
    // and the `Click` arm below is then unreachable code. That much is read
    // straight off the crate: no options means no `needs_response`, which means
    // the call is told not to wait and returns the empty result it started with.
    let mut options = Notification::new();
    options.wait_for_click(true);

    // That waiting is why this can't be the scheduler thread: the call doesn't
    // return until the user has done something, which may be hours away or
    // never. This thread exists to wait on one notification and then end. Only
    // one reminder a day is shown, so only one such thread a day is parked —
    // each with a half-second poll on the main run loop behind it, which is
    // what a click arriving long after the banner has gone costs.
    std::thread::spawn(move || {
        match send_notification(REMINDER_TITLE, None, REMINDER_BODY, Some(&options)) {
            Ok(NotificationResponse::Click) => on_click(),
            // Dismissed: the day is skipped and Reflect says nothing more
            // about it.
            //
            // Whether a banner nobody touches counts as dismissed is a guess,
            // and stated as one. The crate calls it dismissed once the
            // notification leaves `deliveredNotifications`, and a timed-out
            // banner is widely said to move to Notification Center rather than
            // leave — which would keep it live, and an evening's reminder
            // opened at midnight would still open the page. Nothing here has
            // been run to confirm that, and it may well turn on the
            // notification style the user has set. Either way the tray is the
            // way back in.
            Ok(_) => {}
            Err(err) => eprintln!("could not show the daily reminder: {err}"),
        }
    });

    Ok(())
}
