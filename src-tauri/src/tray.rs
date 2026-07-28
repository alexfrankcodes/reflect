//! Builds the tray/menu-bar icon and routes clicks on it.
//!
//! The menu's shape lives in `reflect_core::tray_menu`; this module is the
//! adapter that turns it into real OS menu items and back again.

use reflect_core::entries::Entries;
use reflect_core::tray_menu::TrayAction;
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager, Runtime,
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
        Some(TrayAction::RevealEntriesFolder) => reveal_entries_folder(app),
        None => {}
    }
}

/// Show the user their entries folder in Finder or Explorer.
///
/// Reflect has no export feature and doesn't need one: the entries are plain
/// files in a folder the OS already knows how to open, copy, and back up. This
/// is the whole of handing them over.
fn reveal_entries_folder<R: Runtime>(app: &AppHandle<R>) {
    let entries = app.state::<Entries>();

    // Asked for rather than assumed: someone who wants to see where their
    // writing will land before they've written any is asking a fair question,
    // and an empty folder answers it.
    let dir = match entries.ensure_dir() {
        Ok(dir) => dir,
        Err(err) => {
            // Neither failure has anywhere useful to go: the tray menu closes
            // on click, and there is no window of ours to put the news in.
            eprintln!("could not create the entries folder: {err}");
            return;
        }
    };

    if let Err(err) = tauri_plugin_opener::open_path(dir, None::<&str>) {
        eprintln!("could not open the entries folder: {err}");
    }
}
