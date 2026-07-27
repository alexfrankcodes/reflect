//! Builds the tray/menu-bar icon and routes clicks on it.
//!
//! The menu's shape lives in `reflect_core::tray_menu`; this module is the
//! adapter that turns it into real OS menu items and back again.

use reflect_core::tray_menu::TrayAction;
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Runtime,
};

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let items = TrayAction::ALL
        .into_iter()
        .map(|action| MenuItem::with_id(app, action.id(), action.label(), true, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let item_refs: Vec<&dyn IsMenuItem<R>> =
        items.iter().map(|i| i as &dyn IsMenuItem<R>).collect();
    let menu = Menu::with_items(app, &item_refs)?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("`bundle.icon` in tauri.conf.json must list at least one icon");

    // Deliberately not `.icon_as_template(true)`: macOS template images throw
    // away colour and tint the alpha mask, which would flatten the current
    // full-colour icon into an undifferentiated silhouette. Turn it on once a
    // purpose-drawn monochrome menu-bar asset exists.
    TrayIconBuilder::with_id("reflect")
        .icon(icon)
        .tooltip("Reflect")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .build(app)?;

    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match TrayAction::from_id(event.id.as_ref()) {
        Some(TrayAction::WriteTodaysReflection) => crate::notes::open_or_report(app),
        Some(TrayAction::OpenSettings) => crate::settings::open_or_report(app),
        Some(TrayAction::BrowseEntries) => crate::browse::open_or_report(app),
        Some(TrayAction::Quit) => {
            if let Err(err) = crate::notes::quit(app) {
                eprintln!("could not close the notes window before quitting: {err}");
                app.exit(0);
            }
        }
        // Wired up by a later ticket: Reveal Entries Folder (#18). Selecting
        // it is a no-op for now.
        Some(TrayAction::RevealEntriesFolder) => {}
        None => {}
    }
}
