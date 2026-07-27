//! What the user can change: when Reflect nudges, and whether the notes page
//! carries a writing prompt.

use std::io;
use std::path::PathBuf;

use chrono::{NaiveTime, Timelike};

use crate::read_if_written;
use crate::schedule::DEFAULT_DAILY_TIME;

/// Everything the Settings window can change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settings {
    /// The time of day the daily reminder goes out.
    pub daily_time: NaiveTime,
    /// Whether the notes page carries a writing prompt at all. Off means no
    /// prompt line, not a blank one — Reflect as a plain journal.
    pub show_prompts: bool,
}

impl Default for Settings {
    /// What Reflect does before anyone has told it otherwise.
    fn default() -> Self {
        Self {
            daily_time: DEFAULT_DAILY_TIME,
            show_prompts: true,
        }
    }
}

/// The settings as they sit on disk: one `key = value` per line, in the same
/// folder as the entries and every bit as readable as they are. Nothing here
/// is worth a config format the user can't open in Notepad.
pub struct SettingsFile {
    path: PathBuf,
}

const DAILY_TIME_KEY: &str = "daily-time";
const SHOW_PROMPTS_KEY: &str = "show-prompts";

/// How a time is written. Seconds are dropped: Reflect's reminder is a time of
/// day someone chose, not an instant.
const TIME_FORMAT: &str = "%H:%M";

impl SettingsFile {
    /// The settings kept at `path`. The file needn't exist yet.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The user's settings, or Reflect's defaults where they haven't said.
    ///
    /// A file that isn't there yet is a first run, and a line that makes no
    /// sense is one setting Reflect can't honour — both answer with the
    /// default for what's missing and nothing more. Trouble actually reading
    /// the file is different, and is returned as the error it is: silently
    /// standing 9pm in place of the time the user set, for as long as the
    /// trouble lasts, would be the worst of the three outcomes.
    pub fn load(&self) -> io::Result<Settings> {
        Ok(read_if_written(&self.path)?.map_or_else(Settings::default, |text| parse(&text)))
    }

    /// Write `settings` out, creating the folder if this is the first time.
    pub fn save(&self, settings: &Settings) -> io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(
            &self.path,
            format!(
                "{DAILY_TIME_KEY} = {}\n{SHOW_PROMPTS_KEY} = {}\n",
                format_daily_time(settings.daily_time),
                if settings.show_prompts { "on" } else { "off" },
            ),
        )
    }
}

/// Write a time of day the way Reflect writes it — in the settings file, and
/// in the box the Settings window shows it in.
///
/// The counterpart to [`parse_daily_time`], and here beside it so the two
/// can't drift: a caller that spells the format out again is one release away
/// from writing a time Reflect can no longer read back.
pub fn format_daily_time(daily_time: NaiveTime) -> String {
    daily_time.format(TIME_FORMAT).to_string()
}

/// Read a time of day as Reflect writes it, and as `<input type="time">` hands
/// it back — which is `HH:MM` on some webviews and `HH:MM:SS` on others.
///
/// `None` for anything else, so that a caller who has been handed a time can
/// say so rather than quietly reaching for a default.
pub fn parse_daily_time(text: &str) -> Option<NaiveTime> {
    let text = text.trim();
    NaiveTime::parse_from_str(text, TIME_FORMAT)
        .or_else(|_| NaiveTime::parse_from_str(text, "%H:%M:%S"))
        .ok()
        // Whatever precision arrived, the reminder is set to a minute.
        .and_then(|time| time.with_second(0))
        .and_then(|time| time.with_nanosecond(0))
}

fn parse(text: &str) -> Settings {
    let mut settings = Settings::default();

    for line in text.lines() {
        // No `=` is no setting: a blank line, a note somebody left at the top
        // of the file, anything at all. Reflect reads what it recognises and
        // steps over the rest.
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        match key.trim().to_ascii_lowercase().as_str() {
            DAILY_TIME_KEY => {
                if let Some(time) = parse_daily_time(value) {
                    settings.daily_time = time;
                }
            }
            SHOW_PROMPTS_KEY => {
                if let Some(on) = parse_switch(value) {
                    settings.show_prompts = on;
                }
            }
            _ => {}
        }
    }

    settings
}

fn parse_switch(text: &str) -> Option<bool> {
    match text.trim().to_ascii_lowercase().as_str() {
        "on" => Some(true),
        "off" => Some(false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveTime;

    use super::*;
    use crate::schedule::DEFAULT_DAILY_TIME;

    /// A wall-clock time these tests can talk about.
    fn time(hour: u32, minute: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(hour, minute, 0).expect("test time must be a real time")
    }

    #[test]
    fn a_first_run_gets_the_evening_nudge_and_its_prompts() {
        let home = tempfile::tempdir().unwrap();

        let settings = SettingsFile::at(home.path().join("settings.txt"))
            .load()
            .unwrap();

        assert_eq!(settings.daily_time, DEFAULT_DAILY_TIME);
        assert!(settings.show_prompts);
    }

    #[test]
    fn what_the_user_set_is_what_they_get_back() {
        let home = tempfile::tempdir().unwrap();
        let file = SettingsFile::at(home.path().join("settings.txt"));
        let chosen = Settings {
            daily_time: time(7, 30),
            show_prompts: false,
        };

        file.save(&chosen).unwrap();

        assert_eq!(file.load().unwrap(), chosen);
    }

    #[test]
    fn settings_are_a_plain_readable_file_anyone_can_open() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.txt");

        SettingsFile::at(&path)
            .save(&Settings {
                daily_time: time(7, 30),
                show_prompts: false,
            })
            .unwrap();

        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "daily-time = 07:30\nshow-prompts = off\n"
        );
    }

    #[test]
    fn the_settings_folder_is_created_rather_than_demanded() {
        let home = tempfile::tempdir().unwrap();
        let file = SettingsFile::at(home.path().join("never/made/settings.txt"));

        file.save(&Settings::default()).unwrap();

        assert_eq!(file.load().unwrap(), Settings::default());
    }

    #[test]
    fn a_line_nobody_can_read_falls_back_only_for_itself() {
        // One mangled setting shouldn't cost the user the other one.
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.txt");
        std::fs::write(&path, "daily-time = half past ten\nshow-prompts = off\n").unwrap();

        let settings = SettingsFile::at(&path).load().unwrap();

        assert_eq!(settings.daily_time, DEFAULT_DAILY_TIME);
        assert!(!settings.show_prompts);
    }

    #[test]
    fn lines_reflect_knows_nothing_about_are_left_alone() {
        // Room for a hand-written note at the top of the file, and for a
        // setting a later version of Reflect wrote and this one hasn't heard
        // of, without either turning into a lost daily time.
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.txt");
        std::fs::write(
            &path,
            "# my settings\ntheme = midnight\n\ndaily-time = 07:30\n",
        )
        .unwrap();

        assert_eq!(
            SettingsFile::at(&path).load().unwrap().daily_time,
            time(7, 30)
        );
    }

    #[test]
    fn a_hand_edited_file_is_read_the_way_it_was_meant() {
        // Whitespace and capitals are what a person typing into Notepad
        // produces, and none of it changes what they meant.
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.txt");
        std::fs::write(&path, "  Daily-Time=07:30  \n\tSHOW-PROMPTS = Off\n").unwrap();

        let settings = SettingsFile::at(&path).load().unwrap();

        assert_eq!(settings.daily_time, time(7, 30));
        assert!(!settings.show_prompts);
    }

    #[test]
    fn a_time_written_out_reads_back_as_the_same_time() {
        assert_eq!(format_daily_time(time(7, 30)), "07:30");
        assert_eq!(
            parse_daily_time(&format_daily_time(time(7, 30))),
            Some(time(7, 30))
        );
    }

    #[test]
    fn a_time_carrying_seconds_is_still_a_time() {
        // `<input type="time">` hands some webviews back `HH:MM:SS`.
        assert_eq!(parse_daily_time("07:30:00"), Some(time(7, 30)));
        assert_eq!(parse_daily_time("07:30"), Some(time(7, 30)));
    }

    #[test]
    fn something_that_isnt_a_time_of_day_is_refused_rather_than_guessed_at() {
        // The Settings window turns this `None` into an error the user sees.
        // Quietly standing a default in its place is how someone's seven
        // o'clock becomes nine without anything saying so.
        assert_eq!(parse_daily_time(""), None);
        assert_eq!(parse_daily_time("7pm"), None);
        assert_eq!(parse_daily_time("25:00"), None);
    }

    #[test]
    fn a_settings_file_that_cannot_be_read_is_an_error_rather_than_a_shrug() {
        // Defaulting here would move the daily reminder to nine in the evening
        // for as long as the trouble lasts, without a word about it. The
        // caller retries; it doesn't want a guess.
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.txt");
        std::fs::create_dir(&path).unwrap();

        assert!(SettingsFile::at(&path).load().is_err());
    }
}
