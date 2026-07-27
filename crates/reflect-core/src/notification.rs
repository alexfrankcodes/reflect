//! What the daily reminder says.
//!
//! Here rather than in the Tauri layer for the same reason the tray menu's
//! labels are: every word a user can read should be spelled out in one place,
//! not scattered across whichever platform happens to display it.

/// The notification's heading — the app, as the OS lists it.
pub const REMINDER_TITLE: &str = "Reflect";

/// The line underneath.
///
/// Deliberately not the day's writing prompt. The prompt belongs on the page,
/// where the user has chosen to be — a prompt on the lock screen is answering
/// a question nobody asked yet, and a user who has turned prompts off
/// shouldn't meet one there at all.
pub const REMINDER_BODY: &str = "Time to write today's reflection.";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reminder_says_something_in_both_of_its_lines() {
        assert!(!REMINDER_TITLE.trim().is_empty());
        assert!(!REMINDER_BODY.trim().is_empty());
    }

    #[test]
    fn the_reminder_gives_nothing_of_the_days_writing_away() {
        // The notification is visible on a lock screen, over someone's
        // shoulder, in a meeting. Whatever it says, it isn't the entry.
        assert!(!REMINDER_BODY.contains('\n'));
        assert!(
            !crate::prompts::DEFAULT_PROMPTS.contains(&REMINDER_BODY),
            "the reminder must not be one of the writing prompts"
        );
    }
}
