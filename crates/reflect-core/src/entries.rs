//! Reflect's storage: one plain-text file per calendar day.

use std::io;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;

use crate::read_if_written;

/// The folder holding one `YYYY-MM-DD.txt` per day the user wrote something.
pub struct Entries {
    dir: PathBuf,
}

/// What a call to [`Entries::save`] did to the day's file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Saved {
    /// The file was created or rewritten.
    Wrote,
    /// The day's file already said exactly this, so it was left untouched —
    /// reopening a finished day and closing it again shouldn't so much as
    /// disturb its timestamp.
    Unchanged,
    /// There was nothing worth keeping, so the day's file was left alone —
    /// an unwritten day is a day with no file at all.
    NothingToSave,
}

impl Entries {
    /// Entries stored in `dir`. The folder needn't exist yet.
    pub fn in_dir(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Persist `content` as `date`'s entry: trimmed, newline-terminated, and
    /// read back by [`load`](Self::load) exactly as it was handed in.
    ///
    /// Blank content is refused rather than written: whether something counts
    /// as an entry is [`NotesSession`](crate::notes::NotesSession)'s call, but
    /// storage won't create an empty file even if asked.
    pub fn save(&self, date: NaiveDate, content: &str) -> io::Result<Saved> {
        let content = content.trim();
        if content.is_empty() {
            return Ok(Saved::NothingToSave);
        }

        let file_text = format!("{content}\n");
        let path = self.path_for(date);
        if read_if_written(&path)?.as_deref() == Some(file_text.as_str()) {
            return Ok(Saved::Unchanged);
        }

        std::fs::create_dir_all(&self.dir)?;
        std::fs::write(&path, file_text)?;
        Ok(Saved::Wrote)
    }

    /// What the user wrote on `date`, or `None` if they didn't.
    pub fn load(&self, date: NaiveDate) -> io::Result<Option<String>> {
        // Trimmed rather than raw, so a file someone re-saved from Notepad
        // (CRLF, stray blank line at the end) still comes back the way it
        // would have from Reflect itself.
        Ok(read_if_written(&self.path_for(date))?.map(|text| text.trim_end().to_owned()))
    }

    /// Every day the user has written on, most recent first — the order the
    /// Browse window lists them in, and the only order it ever wants them in.
    pub fn dates(&self) -> io::Result<Vec<NaiveDate>> {
        // A folder that isn't there is the same answer as one holding nothing:
        // nothing makes it until the user asks for it, by saving a first entry
        // or by opening it from the tray, so Browse on a fresh install is an
        // empty list rather than a complaint about a missing folder.
        let listing = match std::fs::read_dir(&self.dir) {
            Ok(listing) => listing,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(err),
        };

        let mut dates: Vec<NaiveDate> = listing
            .filter_map(Result::ok)
            // A folder named like an entry is not one, and neither is a file
            // with nothing in it — Reflect never writes an empty entry, so a
            // day is only a day if there is something on it. Listing either
            // would offer a day that fails the moment it's clicked.
            .filter(|file| matches!(file.metadata(), Ok(day) if day.is_file() && day.len() > 0))
            .filter_map(|file| entry_date(&file.file_name().to_string_lossy()))
            .collect();

        dates.sort_unstable_by(|a, b| b.cmp(a));
        Ok(dates)
    }

    /// The folder itself, made if it isn't there yet.
    ///
    /// Somewhere to point the OS's file browser at. Every other call here
    /// treats a missing folder as a folder with nothing in it, because until
    /// the first save there is nothing to keep — but a folder that isn't
    /// there is one "Reveal Entries Folder" can't open, so asking for it is
    /// what brings it into being.
    pub fn create_dir(&self) -> io::Result<&Path> {
        std::fs::create_dir_all(&self.dir)?;
        Ok(&self.dir)
    }

    fn path_for(&self, date: NaiveDate) -> PathBuf {
        self.dir.join(format!("{}.txt", format_entry_date(date)))
    }
}

/// How a date is written as an entry's file name.
const DATE_FORMAT: &str = "%Y-%m-%d";

/// Write a date the way Reflect names a file after it.
///
/// The counterpart to [`parse_entry_date`], and here beside it so the two can't
/// drift: the Browse window hands a date back as this same string, and a caller
/// that spells the format out again is one release away from asking for a day
/// whose file it can no longer find.
pub fn format_entry_date(date: NaiveDate) -> String {
    date.format(DATE_FORMAT).to_string()
}

/// Read a date as Reflect writes it. `None` for anything else.
pub fn parse_entry_date(text: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(text.trim(), DATE_FORMAT).ok()
}

/// The day `file_name` holds an entry for, or `None` if it isn't one of
/// Reflect's files at all.
fn entry_date(file_name: &str) -> Option<NaiveDate> {
    let named = file_name.strip_suffix(".txt")?;
    let date = parse_entry_date(named)?;

    // A day is a file Reflect itself would have named that way. `2026-7-4.txt`
    // reads as a date but isn't one Reflect wrote, and listing it would offer
    // a day whose file [`Entries::path_for`] then looks for under the name it
    // would have used — and doesn't find.
    (format_entry_date(date) == named).then_some(date)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day;

    #[test]
    fn a_written_entry_lands_in_a_file_named_for_its_day() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        entries
            .save(day(2026, 7, 24), "Rain all afternoon. I liked it.")
            .unwrap();

        let written = std::fs::read_to_string(home.path().join("entries/2026-07-24.txt")).unwrap();
        assert_eq!(written.trim(), "Rain all afternoon. I liked it.");
    }

    #[test]
    fn a_day_with_nothing_on_it_leaves_no_file_behind() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        assert_eq!(
            entries.save(day(2026, 7, 24), "   \n\t  ").unwrap(),
            Saved::NothingToSave
        );
        assert!(!home.path().join("entries/2026-07-24.txt").exists());
    }

    #[test]
    fn saving_the_very_same_words_again_does_not_touch_the_file() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let path = home.path().join("entries/2026-07-24.txt");
        entries
            .save(day(2026, 7, 24), "Same words as before.")
            .unwrap();

        // Read-only is how the test *observes* an untouched file: a rewrite
        // would fail loudly here, where an equal mtime could just be a clock
        // too coarse to notice.
        set_readonly(&path, true);
        let outcome = entries.save(day(2026, 7, 24), "Same words as before.");
        set_readonly(&path, false);

        assert_eq!(outcome.unwrap(), Saved::Unchanged);
    }

    #[test]
    fn what_was_written_on_a_day_reads_back_word_for_word() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        entries
            .save(day(2026, 7, 24), "Two lines.\nThe second one.")
            .unwrap();

        assert_eq!(
            entries.load(day(2026, 7, 24)).unwrap().as_deref(),
            Some("Two lines.\nThe second one.")
        );
    }

    #[test]
    fn an_entry_file_ends_with_a_newline_like_any_other_text_file() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        entries.save(day(2026, 7, 24), "One line.").unwrap();

        let written = std::fs::read_to_string(home.path().join("entries/2026-07-24.txt")).unwrap();
        assert_eq!(written, "One line.\n");
    }

    #[test]
    fn the_days_written_on_come_back_most_recent_first() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        for date in [day(2026, 7, 24), day(2025, 12, 31), day(2026, 7, 25)] {
            entries.save(date, "Something.").unwrap();
        }

        assert_eq!(
            entries.dates().unwrap(),
            vec![day(2026, 7, 25), day(2026, 7, 24), day(2025, 12, 31)]
        );
    }

    #[test]
    fn before_anyone_has_written_a_word_there_are_no_days_to_list() {
        // The entries folder isn't made until the first save, so Browse on a
        // fresh install is an empty list rather than an error about a missing
        // folder.
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        assert_eq!(entries.dates().unwrap(), Vec::new());
    }

    #[test]
    fn whatever_else_is_in_the_folder_is_not_a_day() {
        // The entries folder is the user's own — it lives somewhere they can
        // open, and Reveal Entries Folder invites them into it. Anything they
        // leave there is theirs, and none of it is a day Reflect wrote.
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join("entries");
        let entries = Entries::in_dir(&dir);
        entries
            .save(day(2026, 7, 24), "The one real entry.")
            .unwrap();
        std::fs::write(dir.join("notes.txt"), "shopping list").unwrap();
        std::fs::write(dir.join("2026-07-25.md"), "written elsewhere").unwrap();
        std::fs::write(dir.join("2026-13-40.txt"), "not a date").unwrap();
        // Reads as a date, but not as Reflect spells one — and a day listed
        // from a name Reflect couldn't rebuild is a day that fails on click.
        std::fs::write(dir.join("2026-7-4.txt"), "hand-named").unwrap();
        std::fs::create_dir(dir.join("2026-07-26.txt")).unwrap();
        // A day with nothing on it is a day with no file — one holding nothing
        // is the same day, however it came to be there.
        std::fs::write(dir.join("2026-07-21.txt"), "").unwrap();

        assert_eq!(entries.dates().unwrap(), vec![day(2026, 7, 24)]);
    }

    #[test]
    fn the_folder_is_there_to_be_opened_before_a_word_has_been_written() {
        // "Reveal Entries Folder" has to hand the OS a folder that exists, and
        // on a fresh install nothing has been saved, so nothing has made one.
        let home = tempfile::tempdir().unwrap();
        let dir = home.path().join("entries");
        let entries = Entries::in_dir(&dir);

        assert_eq!(entries.create_dir().unwrap(), dir);
        assert!(dir.is_dir());
    }

    #[test]
    fn opening_the_folder_leaves_everything_already_written_in_it() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        entries.save(day(2026, 7, 24), "Still here.").unwrap();

        entries.create_dir().unwrap();

        assert_eq!(
            entries.load(day(2026, 7, 24)).unwrap().as_deref(),
            Some("Still here.")
        );
    }

    #[test]
    fn a_day_never_written_on_reads_back_as_nothing() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        assert_eq!(entries.load(day(2026, 7, 24)).unwrap(), None);
    }

    fn set_readonly(path: &Path, readonly: bool) {
        let mut perms = std::fs::metadata(path).unwrap().permissions();
        perms.set_readonly(readonly);
        std::fs::set_permissions(path, perms).unwrap();
    }
}
