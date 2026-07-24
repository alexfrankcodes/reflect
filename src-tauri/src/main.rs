// Prevents an extra console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod tray;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            // Reflect has no Dock presence on macOS — the menu bar item is the
            // whole of its persistent UI. On Windows the equivalent falls out
            // of never opening a window at startup (`app.windows` is empty in
            // tauri.conf.json), so there is nothing to put in the taskbar.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

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
