//! Reflect's storage: one plain-text file per calendar day.

use std::io;
use std::path::PathBuf;

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

    fn path_for(&self, date: NaiveDate) -> PathBuf {
        self.dir.join(format!("{}.txt", date.format("%Y-%m-%d")))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

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
