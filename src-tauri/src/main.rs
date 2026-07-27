// Prevents an extra console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod notes;
mod notify;
mod reminder;
mod tray;

use tauri::Manager;

/// Every way into the notes window that isn't the tray menu ends up here.
fn open_notes<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Err(err) = notes::open(app) {
        eprintln!("could not open the notes window: {err}");
    }
}

fn main() {
    tauri::Builder::default()
        // Registered first, as the plugin requires. A tray app must never run
        // twice — and the second launch this catches is usually not a user
        // starting Reflect again but Windows opening the `reflect://` link of
        // a clicked reminder, which belongs to the copy already running.
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            open_notes(app);
        }))
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![
            notes::notes_page,
            notes::notes_close
        ])
        .setup(|app| {
            // Reflect has no Dock presence on macOS — the menu bar item is the
            // whole of its persistent UI. On Windows the equivalent falls out
            // of never opening a window at startup (`app.windows` is empty in
            // tauri.conf.json), so there is nothing to put in the taskbar.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Asked for rather than hardcoded, so entries land wherever this OS
            // keeps per-app data — `%APPDATA%\<identifier>\entries` on Windows,
            // `~/Library/Application Support/<identifier>/entries` on macOS.
            let data_dir = app.path().app_data_dir()?;
            app.manage(notes::Notes::with_entries_in(data_dir.join("entries")));

            use tauri_plugin_deep_link::DeepLinkExt;

            // The scheme is written into the registry by the installer, but a
            // `cargo run` was never installed — so ask for it at startup, and
            // a clicked reminder has somewhere to arrive in development too.
            #[cfg(any(target_os = "windows", target_os = "linux"))]
            if let Err(err) = app.deep_link().register_all() {
                eprintln!("could not register the reflect:// link: {err}");
            }

            let handle = app.handle().clone();
            app.deep_link().on_open_url(move |_event| {
                // The only link Reflect registers is the one its own reminder
                // carries, so any arrival means the same thing.
                open_notes(&handle);
            });

            tray::create(app.handle())?;
            reminder::start(app.handle(), data_dir.join("last-reminder.txt"));
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to start Reflect")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = event {
                // A tray-only app outlives its windows. Tauri asks to exit once
                // the last one closes; only an explicit Quit (which carries an
                // exit code) should actually take the app down.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
