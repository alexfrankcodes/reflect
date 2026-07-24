//! One sitting at the notes window: opened on a date, closed with some text.

use std::io;

use chrono::NaiveDate;

use crate::entries::{Entries, Saved};
use crate::prompts::prompt_for_date;

/// The notes window's state for a single day, from open to close.
pub struct NotesSession {
    date: NaiveDate,
    prompt: &'static str,
    opened_with: String,
}

impl NotesSession {
    /// Open the page for `date`: its prompt, and whatever is already written.
    pub fn open(entries: &Entries, date: NaiveDate) -> io::Result<Self> {
        Ok(Self {
            date,
            prompt: prompt_for_date(date),
            opened_with: entries.load(date)?.unwrap_or_default(),
        })
    }

    /// The line shown above the textarea.
    pub fn prompt(&self) -> &str {
        self.prompt
    }

    /// The text the textarea starts out holding — empty on a fresh day,
    /// otherwise the day's entry, so that reopening picks up where the user
    /// left off rather than staring past what they already wrote.
    pub fn opened_with(&self) -> &str {
        &self.opened_with
    }

    /// Close the window on `text`, keeping it only if the user actually wrote
    /// something. There is no save button and no "discard?" dialog — this call
    /// is the whole of Reflect's save decision.
    ///
    /// A window closed on nothing is a day skipped, and leaves no file. The
    /// prompt counts as nothing: Reflect's own window keeps the prompt out of
    /// the textarea, but a day should never be recorded as "written" on words
    /// the user didn't write, however the text got there.
    ///
    /// Clearing an entry and closing is *not* a delete — the day's file stays
    /// as it was. Reflect only ever adds to the record.
    pub fn close(&self, entries: &Entries, text: &str) -> io::Result<Saved> {
        let written = text.trim();
        if written.is_empty() || written == self.prompt.trim() {
            return Ok(Saved::NothingToSave);
        }

        entries.save(self.date, written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date must be a real date")
    }

    #[test]
    fn a_fresh_day_opens_on_that_days_prompt_and_a_blank_page() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        let session = NotesSession::open(&entries, day(2026, 7, 24)).unwrap();

        assert_eq!(session.prompt(), prompt_for_date(day(2026, 7, 24)));
        assert_eq!(session.opened_with(), "");
    }

    #[test]
    fn closing_on_something_written_keeps_it() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let session = NotesSession::open(&entries, day(2026, 7, 24)).unwrap();

        let outcome = session
            .close(&entries, "Walked the long way home.")
            .unwrap();

        assert!(matches!(outcome, Saved::Wrote(_)));
        assert_eq!(
            entries.load(day(2026, 7, 24)).unwrap().as_deref(),
            Some("Walked the long way home.")
        );
    }

    #[test]
    fn closing_an_untouched_window_keeps_nothing() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let session = NotesSession::open(&entries, day(2026, 7, 24)).unwrap();

        let outcome = session.close(&entries, "").unwrap();

        assert_eq!(outcome, Saved::NothingToSave);
        assert_eq!(entries.load(day(2026, 7, 24)).unwrap(), None);
    }

    #[test]
    fn a_window_holding_nothing_but_its_own_prompt_counts_as_untouched() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let session = NotesSession::open(&entries, day(2026, 7, 24)).unwrap();

        let prompt = session.prompt().to_owned();
        let outcome = session.close(&entries, &prompt).unwrap();

        assert_eq!(outcome, Saved::NothingToSave);
        assert_eq!(entries.load(day(2026, 7, 24)).unwrap(), None);
    }

    #[test]
    fn reopening_a_finished_day_and_closing_it_again_changes_nothing() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        NotesSession::open(&entries, day(2026, 7, 24))
            .unwrap()
            .close(&entries, "Written this morning.")
            .unwrap();

        let reopened = NotesSession::open(&entries, day(2026, 7, 24)).unwrap();
        let text = reopened.opened_with().to_owned();
        let outcome = reopened.close(&entries, &text).unwrap();

        assert_eq!(text, "Written this morning.");
        assert_eq!(outcome, Saved::Unchanged);
    }
}
