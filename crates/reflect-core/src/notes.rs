//! One sitting at the notes window: opened on a date, closed with some text.

use std::io;

use chrono::NaiveDate;

use crate::entries::{Entries, Saved};
use crate::prompts::prompt_for_date;
use crate::settings::Settings;

/// The notes window's state for a single day, from open to close.
pub struct NotesSession {
    date: NaiveDate,
    prompt: Option<&'static str>,
    opened_with: String,
}

impl NotesSession {
    /// Open the page for `date`: its prompt, and whatever is already written.
    pub fn open(entries: &Entries, settings: &Settings, date: NaiveDate) -> io::Result<Self> {
        Ok(Self {
            date,
            prompt: settings.show_prompts.then(|| prompt_for_date(date)),
            opened_with: entries.load(date)?.unwrap_or_default(),
        })
    }

    /// The line shown above the textarea, or `None` where the user has turned
    /// prompts off — which means no line at all rather than an empty one.
    pub fn prompt(&self) -> Option<&'static str> {
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
    /// A window closed on nothing is a day skipped, and leaves no file. A
    /// prompt that was shown counts as nothing: Reflect's own window keeps the
    /// prompt out of the textarea, but a day should never be recorded as
    /// "written" on words the user didn't write, however the text got there.
    /// Where no prompt was shown, that reasoning doesn't apply — Reflect put
    /// nothing on the page, so everything on it was typed.
    ///
    /// Clearing an entry and closing is *not* a delete — the day's file stays
    /// as it was. Reflect only ever adds to the record.
    pub fn close(&self, entries: &Entries, text: &str) -> io::Result<Saved> {
        let written = text.trim();
        if written.is_empty() || self.prompt.is_some_and(|prompt| written == prompt.trim()) {
            return Ok(Saved::NothingToSave);
        }

        entries.save(self.date, written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::day;

    /// Reflect as it comes out of the box: prompts on.
    fn with_prompts() -> Settings {
        Settings::default()
    }

    /// Reflect as a plain journal.
    fn without_prompts() -> Settings {
        Settings {
            show_prompts: false,
            ..Settings::default()
        }
    }

    #[test]
    fn a_fresh_day_opens_on_that_days_prompt_and_a_blank_page() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        let session = NotesSession::open(&entries, &with_prompts(), day(2026, 7, 24)).unwrap();

        assert_eq!(session.prompt(), Some(prompt_for_date(day(2026, 7, 24))));
        assert_eq!(session.opened_with(), "");
    }

    #[test]
    fn prompts_turned_off_leaves_the_page_with_no_prompt_at_all() {
        // Not a blank prompt line — none. The page is a plain journal, and
        // the window has nothing above the textarea to draw.
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));

        let session = NotesSession::open(&entries, &without_prompts(), day(2026, 7, 24)).unwrap();

        assert_eq!(session.prompt(), None);
    }

    #[test]
    fn turning_prompts_back_on_restores_the_day_its_own_prompt() {
        // The prompt belongs to the date, not to the session, so a day
        // reopened with prompts back on shows the line it always would have.
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        NotesSession::open(&entries, &without_prompts(), day(2026, 7, 24)).unwrap();

        let reopened = NotesSession::open(&entries, &with_prompts(), day(2026, 7, 24)).unwrap();

        assert_eq!(reopened.prompt(), Some(prompt_for_date(day(2026, 7, 24))));
    }

    #[test]
    fn with_prompts_off_a_prompts_words_are_the_users_own() {
        // The "that's only the prompt" rule exists to stop Reflect recording a
        // day on words it put there itself. With prompts off it put nothing
        // there, so whatever is on the page was typed.
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let session = NotesSession::open(&entries, &without_prompts(), day(2026, 7, 24)).unwrap();

        let outcome = session
            .close(&entries, prompt_for_date(day(2026, 7, 24)))
            .unwrap();

        assert_eq!(outcome, Saved::Wrote);
    }

    #[test]
    fn closing_on_something_written_keeps_it() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let session = NotesSession::open(&entries, &with_prompts(), day(2026, 7, 24)).unwrap();

        let outcome = session
            .close(&entries, "Walked the long way home.")
            .unwrap();

        assert_eq!(outcome, Saved::Wrote);
        assert_eq!(
            entries.load(day(2026, 7, 24)).unwrap().as_deref(),
            Some("Walked the long way home.")
        );
    }

    #[test]
    fn closing_an_untouched_window_keeps_nothing() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let session = NotesSession::open(&entries, &with_prompts(), day(2026, 7, 24)).unwrap();

        let outcome = session.close(&entries, "").unwrap();

        assert_eq!(outcome, Saved::NothingToSave);
        assert_eq!(entries.load(day(2026, 7, 24)).unwrap(), None);
    }

    #[test]
    fn a_window_holding_nothing_but_its_own_prompt_counts_as_untouched() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        let session = NotesSession::open(&entries, &with_prompts(), day(2026, 7, 24)).unwrap();

        let prompt = session.prompt().expect("prompts are on in this session");
        let outcome = session.close(&entries, prompt).unwrap();

        assert_eq!(outcome, Saved::NothingToSave);
        assert_eq!(entries.load(day(2026, 7, 24)).unwrap(), None);
    }

    #[test]
    fn reopening_a_finished_day_and_closing_it_again_changes_nothing() {
        let home = tempfile::tempdir().unwrap();
        let entries = Entries::in_dir(home.path().join("entries"));
        NotesSession::open(&entries, &with_prompts(), day(2026, 7, 24))
            .unwrap()
            .close(&entries, "Written this morning.")
            .unwrap();

        let reopened = NotesSession::open(&entries, &with_prompts(), day(2026, 7, 24)).unwrap();
        let text = reopened.opened_with().to_owned();
        let outcome = reopened.close(&entries, &text).unwrap();

        assert_eq!(text, "Written this morning.");
        assert_eq!(outcome, Saved::Unchanged);
    }
}
