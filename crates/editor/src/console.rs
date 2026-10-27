//! The console: a bounded log of what the editor reported — the play session's script
//! errors and the external tools' launch results — each line with its severity and, where
//! it names a file, the request a click on it files. The panel's drawing is
//! `editor_integration`'s; the host writes a line here and on the status bar through one
//! function, so neither surface misses a line the other shows.

use std::collections::VecDeque;

use crate::external_tools::OpenRequest;

/// The most entries the log keeps; the oldest is dropped first.
pub const CONSOLE_CAPACITY: usize = 500;

/// How grave a console line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Info,
    /// Drawn with an "Error: " prefix in the status bar's error colour
    Error,
}

/// One entry: a line, or a run of lines that collapsed into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleEntry {
    /// Names the entry for the log's whole life, so a click pressed on it and released
    /// after the log moved opens this entry or nothing, never the one now in its place
    pub id: usize,
    pub severity: Severity,
    /// The newest text of the lines that collapsed into the entry
    pub text: String,
    /// What a click on the entry opens; `None` for a line that names no file
    pub location: Option<OpenRequest>,
    /// How many lines collapsed into the entry, one for a single line
    pub count: usize,
}

/// The log itself, at most [`CONSOLE_CAPACITY`] entries, oldest first.
#[derive(Debug, Default)]
pub struct ConsoleLog {
    entries: VecDeque<ConsoleEntry>,
    next_id: usize,
}

impl ConsoleLog {
    /// Append a line. It collapses into the last entry — counting up and keeping the
    /// newest text — when the two share a severity, a location and a text with every run
    /// of digits taken as one, so an error repeated each frame is one entry whatever
    /// number it carries; the location's line is compared whole. Returns whether the
    /// oldest entry was dropped to make room.
    pub fn push(&mut self, severity: Severity, text: impl Into<String>, location: Option<OpenRequest>) -> bool {
        let text = text.into();
        if let Some(last) = self.entries.back_mut() {
            if last.severity == severity && last.location == location && same_but_numbers(&last.text, &text) {
                last.text = text;
                last.count += 1;
                return false;
            }
        }
        let full = self.entries.len() >= CONSOLE_CAPACITY;
        if full {
            self.entries.pop_front();
        }
        self.entries.push_back(ConsoleEntry { id: self.next_id, severity, text, location, count: 1 });
        self.next_id += 1;
        full
    }

    pub fn entries(&self) -> &VecDeque<ConsoleEntry> {
        &self.entries
    }

    pub fn get(&self, index: usize) -> Option<&ConsoleEntry> {
        self.entries.get(index)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// Whether two texts match once every run of digits in each is taken as one mark.
fn same_but_numbers(first: &str, second: &str) -> bool {
    digit_runs_marked(first).eq(digit_runs_marked(second))
}

/// The text's characters with each run of ASCII digits replaced by one `None`.
fn digit_runs_marked(text: &str) -> impl Iterator<Item = Option<char>> + '_ {
    let mut previous_was_digit = false;
    text.chars().filter_map(move |character| {
        let is_digit = character.is_ascii_digit();
        let continues_run = is_digit && previous_was_digit;
        previous_was_digit = is_digit;
        match (is_digit, continues_run) {
            (true, true) => None,
            (true, false) => Some(None),
            (false, _) => Some(Some(character)),
        }
    })
}

/// One arrow key's step through the entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsoleStep {
    Up,
    Down,
}

/// The console panel's state (a field on `EditorContext`): the log, its scroll and its
/// keyboard. The asset browser's keyboard model: a press in the panel takes the keyboard,
/// and while the console holds it, Up and Down walk the entries and Enter opens the
/// focused one.
#[derive(Debug, Default)]
pub struct ConsoleState {
    pub log: ConsoleLog,
    /// Vertical scroll (shared panel pattern)
    pub scroll: crate::ScrollState,
    /// Whether the console holds the keyboard
    pub holds_keyboard: bool,
    /// The entry the arrows walk, ringed while the console holds the keyboard
    pub focused: Option<usize>,
    /// Scroll the focused entry wholly into view at the next render
    pub reveal_focused: bool,
    /// The view stands above the newest entry, so a new line leaves it where it is; at the
    /// bottom it follows each line in
    pub scrolled_up: bool,
    /// Entries the log dropped since the panel last drew: each moved every row up one, so a
    /// scrolled-up view moves up as far to keep the lines it shows in place
    pub rows_dropped: usize,
}

impl ConsoleState {
    /// Append a line to the log; the focus stays on its entry when the oldest is dropped.
    pub fn push(&mut self, severity: Severity, text: impl Into<String>, location: Option<OpenRequest>) {
        if self.log.push(severity, text, location) {
            self.focused = self.focused.and_then(|index| index.checked_sub(1));
            self.rows_dropped += 1;
        }
    }

    /// Move the focus one entry, held inside the log. With nothing focused, Up focuses the
    /// newest entry — the one just reported — and Down the oldest. The entry is scrolled
    /// into view at the next render.
    pub fn step_focus(&mut self, step: ConsoleStep) {
        let Some(last) = self.log.len().checked_sub(1) else {
            return;
        };
        self.focused = Some(match (self.focused.filter(|&index| index <= last), step) {
            (None, ConsoleStep::Up) => last,
            (None, ConsoleStep::Down) => 0,
            (Some(index), ConsoleStep::Up) => index.saturating_sub(1),
            (Some(index), ConsoleStep::Down) => (index + 1).min(last),
        });
        self.reveal_focused = true;
    }

    /// The request the focused entry files — Enter's, the click's twin.
    pub fn focused_request(&self) -> Option<OpenRequest> {
        self.focused.and_then(|index| self.log.get(index)).and_then(|entry| entry.location.clone())
    }

    /// Empty the log, and the focus with it.
    pub fn clear(&mut self) {
        self.log.clear();
        self.focused = None;
    }

    pub fn release_keyboard(&mut self) {
        self.holds_keyboard = false;
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::external_tools::Location;

    fn at_line(line: usize) -> Option<OpenRequest> {
        Some(OpenRequest { path: PathBuf::from("scripts/paddle.rhai"), location: Location::Line { line, column: None } })
    }

    #[test]
    fn the_line_past_the_capacity_drops_the_oldest_entry() {
        let mut log = ConsoleLog::default();
        for index in 0..CONSOLE_CAPACITY {
            assert!(!log.push(Severity::Info, format!("line {index}"), at_line(index + 1)));
        }
        assert!(log.push(Severity::Info, "the last", None), "the 501st line drops one");
        assert_eq!(log.len(), CONSOLE_CAPACITY);
        assert_eq!(log.get(0).map(|entry| entry.text.as_str()), Some("line 1"), "the first went");
        assert_eq!(log.get(CONSOLE_CAPACITY - 1).map(|entry| entry.text.as_str()), Some("the last"));
    }

    #[test]
    fn lines_differing_only_in_their_numbers_at_one_location_collapse_keeping_the_newest_text() {
        let mut log = ConsoleLog::default();
        log.push(Severity::Error, "frame 12", at_line(4));
        log.push(Severity::Error, "frame 13", at_line(4));
        assert_eq!(
            log.entries().iter().cloned().collect::<Vec<_>>(),
            vec![ConsoleEntry { id: 0, severity: Severity::Error, text: "frame 13".to_string(), location: at_line(4), count: 2 }]
        );
    }

    #[test]
    fn another_text_another_line_or_another_severity_starts_an_entry() {
        let mut log = ConsoleLog::default();
        log.push(Severity::Error, "frame 12", at_line(4));
        log.push(Severity::Error, "a different message", at_line(4));
        log.push(Severity::Error, "a different message", at_line(5));
        log.push(Severity::Info, "a different message", at_line(5));
        assert_eq!(log.len(), 4, "the location's line is never normalized, nor the severity ignored");
        assert!(log.entries().iter().all(|entry| entry.count == 1));
    }

    #[test]
    fn a_line_matching_an_earlier_entry_but_not_the_last_starts_an_entry() {
        let mut log = ConsoleLog::default();
        log.push(Severity::Error, "frame 1", at_line(4));
        log.push(Severity::Info, "Opened paddle.rhai in RustRover", None);
        log.push(Severity::Error, "frame 2", at_line(4));
        assert_eq!(log.len(), 3);
    }

    #[test]
    fn clear_empties_the_log_and_drops_the_focus() {
        let mut console = ConsoleState::default();
        console.push(Severity::Error, "one", at_line(1));
        console.push(Severity::Error, "another", at_line(2));
        console.step_focus(ConsoleStep::Up);
        console.clear();
        assert!(console.log.is_empty());
        assert_eq!(console.focused, None);
        assert_eq!(console.focused_request(), None);
    }

    #[test]
    fn the_arrows_walk_the_entries_inside_the_log_and_the_focus_follows_its_entry_past_an_eviction() {
        let mut console = ConsoleState::default();
        for line in 1..=3 {
            console.push(Severity::Error, format!("error {line}"), at_line(line));
            console.push(Severity::Info, "a launch", None);
        }
        console.step_focus(ConsoleStep::Up);
        assert_eq!(console.focused, Some(5), "Up first focuses the newest entry");
        console.focused = Some(0);
        console.step_focus(ConsoleStep::Up);
        assert_eq!(console.focused, Some(0), "held at the top");
        console.step_focus(ConsoleStep::Down);
        console.step_focus(ConsoleStep::Down);
        assert_eq!(console.focused_request(), at_line(2));

        for index in 0..CONSOLE_CAPACITY - 6 {
            console.push(Severity::Info, format!("filler {index}"), at_line(100 + index));
        }
        console.push(Severity::Info, "the one that drops the oldest", None);
        assert_eq!(console.focused, Some(1));
        assert_eq!(console.focused_request(), at_line(2), "the focus stays on its entry");
    }
}
