// Prevents an extra console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod notes;
mod tray;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
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
            let entries_dir = app.path().app_data_dir()?.join("entries");
            app.manage(notes::Notes::with_entries_in(entries_dir));

            tray::create(app.handle())?;
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
