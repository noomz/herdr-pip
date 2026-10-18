//! Picker: the candidate source panes, attention-first, with type-to-filter.

use crate::herdr::{Snapshot, Source};

/// A snapshot of candidate sources, refreshed each time the picker is shown.
pub struct Picker {
    sources: Vec<Source>,
    pub filter: String,
    /// Index into `matches()`.
    selected: usize,
    /// First match shown at the top of the list.
    offset: usize,
}

impl Picker {
    /// Every pane except the focused one and `own_pane`, attention-first then most recent.
    pub fn new(snap: &Snapshot, own_pane: Option<&str>) -> Self {
        let mut sources: Vec<Source> = snap
            .sources
            .iter()
            .filter(|s| Some(s.pane_id.as_str()) != snap.focused_pane_id.as_deref())
            .filter(|s| Some(s.pane_id.as_str()) != own_pane)
            .cloned()
            .collect();
        // Stable: panes that tie keep the snapshot's workspace/tab order.
        sources.sort_by_key(|s| std::cmp::Reverse((attention(s.status.as_deref()), s.state_change_seq)));
        Picker { sources, filter: String::new(), selected: 0, offset: 0 }
    }

    fn matches(&self) -> Vec<&Source> {
        let filter = self.filter.to_lowercase();
        self.sources
            .iter()
            .filter(|s| {
                filter.is_empty()
                    || s.name.to_lowercase().contains(&filter)
                    || s.status.as_deref().is_some_and(|status| status.contains(&filter))
            })
            .collect()
    }

    pub fn selected(&self) -> Option<&Source> {
        self.matches().get(self.selected).copied()
    }

    pub fn move_by(&mut self, delta: isize) {
        let last = self.matches().len().saturating_sub(1);
        self.selected = self.selected.saturating_add_signed(delta).min(last);
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.selected = 0;
        self.offset = 0;
    }

    /// The source on list row `row` (0 = first row below the top row), as last laid out.
    pub fn at_row(&self, row: usize) -> Option<&Source> {
        self.matches().get(self.offset + row).copied()
    }

    /// `rows` list lines of at most `cols` cells, the selection shown in reverse video.
    /// Scrolls just enough to keep the selection visible.
    pub fn lines(&mut self, cols: u16, rows: u16) -> Vec<Vec<u8>> {
        let rows = usize::from(rows);
        let matches = self.matches();
        if matches.is_empty() {
            let text = if self.sources.is_empty() { "no other panes" } else { "no match" };
            return vec![format!("\x1b[2m  {text}").into_bytes()];
        }
        let offset = if self.selected < self.offset {
            self.selected
        } else if rows > 0 && self.selected >= self.offset + rows {
            self.selected + 1 - rows
        } else {
            self.offset
        };
        let lines = matches
            .iter()
            .enumerate()
            .skip(offset)
            .take(rows)
            .map(|(i, source)| {
                let text = row_text(source, usize::from(cols));
                let style = if i == self.selected { "\x1b[7m" } else { "" };
                format!("{style}{text}").into_bytes()
            })
            .collect();
        self.offset = offset;
        lines
    }
}

/// herdr's attention order: blocked, done (finished, unseen), working, idle, plain shell.
fn attention(status: Option<&str>) -> u8 {
    match status {
        Some("blocked") => 4,
        Some("done") => 3,
        Some("working") => 2,
        Some("idle") => 1,
        _ => 0,
    }
}

/// `! ws/tab/label · status`, padded to `cols` so reverse video spans the row.
fn row_text(source: &Source, cols: usize) -> String {
    let marker = match source.status.as_deref() {
        Some("blocked") => '!',
        Some("done") => '*',
        _ => ' ',
    };
    let mut text = format!("{marker} {}", source.name);
    if let Some(status) = &source.status {
        text = format!("{text} · {status}");
    }
    let mut line: String = text.chars().take(cols).collect();
    let pad = cols.saturating_sub(line.chars().count());
    line.extend(std::iter::repeat_n(' ', pad));
    line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(id: &str, status: Option<&str>, seq: u64) -> Source {
        Source {
            pane_id: id.into(),
            name: format!("ws/tab/{id}"),
            status: status.map(str::to_string),
            state_change_seq: seq,
            cols: 80,
            rows: 24,
        }
    }

    fn snapshot(sources: Vec<Source>, focused: &str) -> Snapshot {
        Snapshot { area: (200, 60), focused_pane_id: Some(focused.into()), sources }
    }

    fn ids(picker: &Picker) -> Vec<&str> {
        picker.matches().iter().map(|s| s.pane_id.as_str()).collect()
    }

    fn text(lines: &[Vec<u8>]) -> Vec<String> {
        lines.iter().map(|line| String::from_utf8_lossy(line).into_owned()).collect()
    }

    #[test]
    fn orders_by_attention_then_recency_and_skips_focused_and_own_pane() {
        let snap = snapshot(
            vec![
                source("shell", None, 0),
                source("idle", Some("idle"), 9),
                source("focused", Some("blocked"), 9),
                source("old-done", Some("done"), 1),
                source("own", None, 0),
                source("working", Some("working"), 5),
                source("blocked", Some("blocked"), 2),
                source("new-done", Some("done"), 7),
            ],
            "focused",
        );
        let picker = Picker::new(&snap, Some("own"));
        assert_eq!(ids(&picker), ["blocked", "new-done", "old-done", "working", "idle", "shell"]);
    }

    #[test]
    fn filter_matches_name_or_status_and_resets_selection() {
        let snap = snapshot(vec![source("api", Some("working"), 0), source("logs", None, 0)], "x");
        let mut picker = Picker::new(&snap, None);
        picker.move_by(1);
        picker.set_filter("LOG".into());
        assert_eq!(ids(&picker), ["logs"]);
        assert_eq!(picker.selected().unwrap().pane_id, "logs");
        picker.set_filter("work".into());
        assert_eq!(ids(&picker), ["api"]);
    }

    #[test]
    fn selection_clamps_and_list_scrolls_to_keep_it_visible() {
        let snap = snapshot((0..5).map(|i| source(&format!("p{i}"), None, 0)).collect(), "x");
        let mut picker = Picker::new(&snap, None);
        picker.move_by(-1);
        assert_eq!(picker.selected().unwrap().pane_id, "p0");
        picker.move_by(10);
        assert_eq!(picker.selected().unwrap().pane_id, "p4");
        let lines = text(&picker.lines(14, 2));
        assert_eq!(lines, ["  ws/tab/p3   ", "\x1b[7m  ws/tab/p4   "]);
        assert_eq!(picker.at_row(0).unwrap().pane_id, "p3");
    }

    #[test]
    fn row_shows_attention_marker_and_status() {
        let line = row_text(&source("claude", Some("blocked"), 0), 40);
        assert_eq!(line.trim_end(), "! ws/tab/claude · blocked");
        assert_eq!(line.chars().count(), 40);
    }

    #[test]
    fn empty_states() {
        let mut picker = Picker::new(&snapshot(vec![source("only", None, 0)], "only"), None);
        assert_eq!(text(&picker.lines(40, 5)), ["\x1b[2m  no other panes"]);
        let mut picker = Picker::new(&snapshot(vec![source("a", None, 0)], "x"), None);
        picker.set_filter("zzz".into());
        assert_eq!(text(&picker.lines(40, 5)), ["\x1b[2m  no match"]);
        assert!(picker.selected().is_none());
    }
}
