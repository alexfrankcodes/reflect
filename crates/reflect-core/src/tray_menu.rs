//! The tray menu, modelled as data.
//!
//! The Tauri layer walks [`TrayAction::ALL`] to build the real menu and routes
//! the string id it gets back through [`TrayAction::from_id`]. Keeping the ids
//! and labels here means a typo is a failing test rather than a menu item that
//! silently does nothing when clicked.

/// Something the user can ask for from the tray menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    /// The daily notification is the way into the notes window; this is the
    /// way in on every other day, when the reminder has been and gone, or was
    /// dismissed, or the user simply wants to write twice.
    WriteTodaysReflection,
    OpenSettings,
    BrowseEntries,
    RevealEntriesFolder,
    Quit,
}

impl TrayAction {
    /// Every action, in the order it appears in the menu.
    pub const ALL: [TrayAction; 5] = [
        TrayAction::WriteTodaysReflection,
        TrayAction::OpenSettings,
        TrayAction::BrowseEntries,
        TrayAction::RevealEntriesFolder,
        TrayAction::Quit,
    ];

    /// Stable identifier handed to the OS menu and echoed back on click.
    pub fn id(self) -> &'static str {
        match self {
            TrayAction::WriteTodaysReflection => "write-todays-reflection",
            TrayAction::OpenSettings => "open-settings",
            TrayAction::BrowseEntries => "browse-entries",
            TrayAction::RevealEntriesFolder => "reveal-entries-folder",
            TrayAction::Quit => "quit",
        }
    }

    /// Text the user sees.
    pub fn label(self) -> &'static str {
        match self {
            TrayAction::WriteTodaysReflection => "Write Today's Reflection",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_reads_top_to_bottom_as_the_spec_describes_it() {
        // Every label a user can see, spelled out in one place.
        let labels: Vec<&str> = TrayAction::ALL.iter().map(|a| a.label()).collect();
        assert_eq!(
            labels,
            vec![
                "Write Today's Reflection",
                "Settings…",
                "Browse Entries",
                "Reveal Entries Folder",
                "Quit Reflect",
            ]
        );
    }

    #[test]
    fn every_menu_id_round_trips_back_to_its_action() {
        for action in TrayAction::ALL {
            assert_eq!(
                TrayAction::from_id(action.id()),
                Some(action),
                "id {:?} did not resolve back to {:?}",
                action.id(),
                action
            );
        }
    }

    #[test]
    fn menu_ids_are_unique() {
        let mut ids: Vec<&str> = TrayAction::ALL.iter().map(|a| a.id()).collect();
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
