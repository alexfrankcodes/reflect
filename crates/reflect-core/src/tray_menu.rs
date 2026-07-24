//! The tray menu, modelled as data.
//!
//! The Tauri layer walks [`entries`] to build the real menu and routes the
//! string id it gets back through [`TrayAction::from_id`]. Keeping the ids and
//! labels here means a typo is a failing test rather than a menu item that
//! silently does nothing when clicked.

/// Something the user can ask for from the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    OpenSettings,
    BrowseEntries,
    RevealEntriesFolder,
    Quit,
}

/// One row of the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrayMenuEntry {
    pub action: TrayAction,
    /// Stable identifier handed to the OS menu and echoed back on click.
    pub id: &'static str,
    /// Text the user sees.
    pub label: &'static str,
}

impl TrayAction {
    /// Every action, in the order it appears in the menu.
    pub const ALL: [TrayAction; 4] = [
        TrayAction::OpenSettings,
        TrayAction::BrowseEntries,
        TrayAction::RevealEntriesFolder,
        TrayAction::Quit,
    ];

    pub fn id(self) -> &'static str {
        match self {
            TrayAction::OpenSettings => "open-settings",
            TrayAction::BrowseEntries => "browse-entries",
            TrayAction::RevealEntriesFolder => "reveal-entries-folder",
            TrayAction::Quit => "quit",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TrayAction::OpenSettings => "Settings…",
            TrayAction::BrowseEntries => "Browse Entries",
            TrayAction::RevealEntriesFolder => "Reveal Entries Folder",
            TrayAction::Quit => "Quit Reflect",
        }
    }

    /// Resolve the id the OS handed back on click. `None` for anything we
    /// didn't put in the menu ourselves.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }
}

/// The tray menu, top to bottom.
pub fn entries() -> Vec<TrayMenuEntry> {
    TrayAction::ALL
        .into_iter()
        .map(|action| TrayMenuEntry {
            action,
            id: action.id(),
            label: action.label(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_lists_every_action_in_display_order() {
        let actions: Vec<TrayAction> = entries().iter().map(|e| e.action).collect();
        assert_eq!(
            actions,
            vec![
                TrayAction::OpenSettings,
                TrayAction::BrowseEntries,
                TrayAction::RevealEntriesFolder,
                TrayAction::Quit,
            ]
        );
    }

    #[test]
    fn menu_labels_match_the_spec() {
        let labels: Vec<&str> = entries().iter().map(|e| e.label).collect();
        assert_eq!(
            labels,
            vec![
                "Settings…",
                "Browse Entries",
                "Reveal Entries Folder",
                "Quit Reflect",
            ]
        );
    }

    #[test]
    fn every_menu_id_round_trips_back_to_its_action() {
        for entry in entries() {
            assert_eq!(
                TrayAction::from_id(entry.id),
                Some(entry.action),
                "id {:?} did not resolve back to {:?}",
                entry.id,
                entry.action
            );
        }
    }

    #[test]
    fn menu_ids_are_unique() {
        let mut ids: Vec<&str> = entries().iter().map(|e| e.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "duplicate tray menu id");
    }

    #[test]
    fn unknown_ids_resolve_to_nothing() {
        assert_eq!(TrayAction::from_id("not-a-menu-item"), None);
        assert_eq!(TrayAction::from_id(""), None);
    }
}
