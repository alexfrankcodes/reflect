//! The two files that between them decide when Reflect nudges, under one lock.
//!
//! Not the Settings window — the state it writes, which the reminder thread and
//! the notes window read. It lives in its own module for that reason: three
//! places reach for it, and only one of them is Settings.

use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

use reflect_core::schedule::LastReminder;
use reflect_core::settings::SettingsFile;

/// The settings the user set, and the record of the last nudge.
///
/// Locked together because they are read and written together, by two threads
/// that disagree about the answer. The reminder thread reads the settings,
/// decides whether a nudge is owed, and records it; the Settings window writes
/// the settings and corrects the record for the new time. Interleave those two
/// and a tick still holding the old time can fire the very reminder the user's
/// change was meant to move — and then record over the correction, so that
/// changing the time is what produces a notification a second later.
pub struct Preferences {
    files: Mutex<Files>,
}

/// The two files, only reachable together. See [`Preferences`].
pub struct Files {
    pub settings: SettingsFile,
    pub last_reminder: LastReminder,
}

impl Preferences {
    pub fn new(settings_path: PathBuf, last_reminder_path: PathBuf) -> Self {
        Self {
            files: Mutex::new(Files {
                settings: SettingsFile::at(settings_path),
                last_reminder: LastReminder::at(last_reminder_path),
            }),
        }
    }

    /// Both files, held exclusively until the guard is dropped. Hold it across
    /// the whole of a read-decide-write, not just the read.
    pub fn lock(&self) -> MutexGuard<'_, Files> {
        self.files.lock().expect("preferences lock")
    }
}
