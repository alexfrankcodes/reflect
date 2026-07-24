//! Builds the tray/menu-bar icon and routes clicks on it.
//!
//! The menu's shape lives in `reflect_core::tray_menu`; this module is the
//! adapter that turns it into real OS menu items and back again.

use reflect_core::tray_menu::{self, TrayAction};
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Runtime,
};

pub fn create<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let items = tray_menu::entries()
        .into_iter()
        .map(|entry| MenuItem::with_id(app, entry.id, entry.label, true, None::<&str>))
        .collect::<tauri::Result<Vec<_>>>()?;
    let item_refs: Vec<&dyn IsMenuItem<R>> =
        items.iter().map(|i| i as &dyn IsMenuItem<R>).collect();
    let menu = Menu::with_items(app, &item_refs)?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("bundle icon is configured in tauri.conf.json");

    TrayIconBuilder::with_id("reflect")
        .icon(icon)
        // On macOS a template image is recoloured by the system, so the icon
        // stays legible in both light and dark menu bars.
        .icon_as_template(true)
        .tooltip("Reflect")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(handle_menu_event)
        .build(app)?;

    Ok(())
}

fn handle_menu_event<R: Runtime>(app: &AppHandle<R>, event: tauri::menu::MenuEvent) {
    match TrayAction::from_id(event.id.as_ref()) {
        Some(TrayAction::Quit) => app.exit(0),
        // Wired up by later tickets: Settings (#16), Browse Entries (#17),
        // Reveal Entries Folder (#18). Selecting them is a no-op for now.
        Some(TrayAction::OpenSettings)
        | Some(TrayAction::BrowseEntries)
        | Some(TrayAction::RevealEntriesFolder) => {}
        None => {}
    }
}
