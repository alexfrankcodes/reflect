//! The writing prompt shown above the notes textarea.

use chrono::{Datelike, NaiveDate};

/// The prompt library Reflect ships with. Short, single-line, open-ended.
pub const DEFAULT_PROMPTS: [&str; 30] = [
    "What's one thing you're grateful for today?",
    "What took more out of you than you expected?",
    "What's something small that went well?",
    "Who did you think about today, and why?",
    "What did you learn about yourself this week?",
    "What would you do differently if today started over?",
    "What are you avoiding right now?",
    "What made you laugh recently?",
    "Where did your attention keep drifting today?",
    "What's one thing you're looking forward to?",
    "What felt harder than it should have?",
    "When did you feel most like yourself today?",
    "What's on your mind that you haven't said out loud?",
    "What did someone do for you lately that you never acknowledged?",
    "What would you tell yourself a year ago?",
    "What's something you've changed your mind about?",
    "What are you carrying that isn't yours to carry?",
    "What did you notice today that you'd usually walk past?",
    "What's the kindest thing you did today?",
    "What's draining your energy at the moment?",
    "What would a good tomorrow look like?",
    "What are you proud of that nobody else knows about?",
    "What's a decision you're circling without making?",
    "What did today ask of you?",
    "Where did you say yes when you meant no?",
    "What do you want more of in your life?",
    "What surprised you this week?",
    "What feels unfinished right now?",
    "Who would you like to reach out to, and what's stopping you?",
    "What's one thing worth remembering about today?",
];

/// Picks the prompt for `date` out of [`DEFAULT_PROMPTS`].
///
/// Seeded by the calendar date rather than randomly, so a day owns its prompt:
/// close the window and reopen it that evening and the same line is waiting.
/// Consecutive days walk the library in order and wrap, so every prompt shows
/// up once before any of them comes round again.
pub fn prompt_for_date(date: NaiveDate) -> &'static str {
    let index = date
        .num_days_from_ce()
        .rem_euclid(DEFAULT_PROMPTS.len() as i32);
    DEFAULT_PROMPTS[index as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date must be a real date")
    }

    #[test]
    fn reopening_the_window_later_the_same_day_shows_the_same_prompt() {
        let today = day(2026, 7, 24);
        assert_eq!(prompt_for_date(today), prompt_for_date(today));
    }

    #[test]
    fn a_full_run_of_days_shows_every_prompt_before_repeating_any() {
        let start = day(2026, 7, 24);
        let run: Vec<&str> = (0..DEFAULT_PROMPTS.len())
            .map(|offset| prompt_for_date(start + chrono::Days::new(offset as u64)))
            .collect();

        let mut seen = run.clone();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            seen.len(),
            DEFAULT_PROMPTS.len(),
            "a prompt repeated inside a single run through the library: {run:?}"
        );
    }

    #[test]
    fn the_library_comes_round_again_after_a_full_run() {
        let start = day(2026, 7, 24);
        let one_full_run_later = start + chrono::Days::new(DEFAULT_PROMPTS.len() as u64);
        assert_eq!(prompt_for_date(start), prompt_for_date(one_full_run_later));
    }

    #[test]
    fn neighbouring_days_get_different_prompts() {
        let today = day(2026, 7, 24);
        let tomorrow = today + chrono::Days::new(1);
        assert_ne!(prompt_for_date(today), prompt_for_date(tomorrow));
    }

    #[test]
    fn dates_before_the_common_era_still_resolve_to_a_prompt() {
        // `num_days_from_ce` goes negative here; plain `%` would index out of
        // bounds. A user with a badly-set system clock should still get a
        // prompt rather than a crash.
        assert!(DEFAULT_PROMPTS.contains(&prompt_for_date(day(-100, 3, 4))));
    }

    #[test]
    fn the_shipped_library_is_thirty_distinct_prompts() {
        let mut prompts = DEFAULT_PROMPTS.to_vec();
        prompts.sort_unstable();
        prompts.dedup();
        assert_eq!(prompts.len(), DEFAULT_PROMPTS.len(), "duplicate prompt");
        assert!(DEFAULT_PROMPTS.iter().all(|p| !p.trim().is_empty()));
    }
}
