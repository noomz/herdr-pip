//! Text mirror: a `herdr terminal session observe` child feeding a local `vt100` grid.

use base64::Engine;
use serde_json::Value;
use std::io::{self, BufRead, BufReader};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// The source's screen as last observed, shared between the reader thread and the painter.
pub struct Mirror {
    parser: vt100::Parser,
    /// Set by every applied frame, cleared by the painter.
    pub dirty: bool,
    /// Set once the stream ends.
    pub closed: bool,
}

impl Mirror {
    fn new(cols: u16, rows: u16) -> Self {
        // One spare column: with a request narrower than the grid, vt100 never joins
        // wrapped rows, so each row's bytes move the cursor only within that row.
        Mirror { parser: vt100::Parser::new(rows, cols + 1, 0), dirty: false, closed: false }
    }

    /// Applies one NDJSON record from the observe stream.
    fn apply(&mut self, line: &str) {
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match record["type"].as_str() {
            Some("terminal.frame") => {
                let bytes = record["bytes"].as_str().unwrap_or("");
                if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(bytes) {
                    self.parser.process(&bytes);
                    self.dirty = true;
                }
            }
            Some("terminal.closed") => self.closed = true,
            _ => {}
        }
    }

    /// The source's screen and its size (columns, rows), without the spare column.
    pub fn screen(&self) -> (&vt100::Screen, (u16, u16)) {
        let screen = self.parser.screen();
        let (rows, grid_cols) = screen.size();
        (screen, (grid_cols - 1, rows))
    }

    /// The left `cols` columns of the last `rows` rows that end at the source's content
    /// (its last non-blank row or the cursor row, whichever is lower), so a short output
    /// still shows instead of the blank rows below it. Each row is self-contained
    /// formatted bytes that start from default attributes at the row's first column.
    pub fn visible_rows(&self, cols: u16, rows: u16) -> Vec<Vec<u8>> {
        let screen = self.parser.screen();
        let grid_cols = screen.size().1;
        let width = cols.min(grid_cols - 1);
        let last_text = screen
            .rows(0, grid_cols)
            .enumerate()
            .filter(|(_, row)| !row.trim().is_empty())
            .last()
            .map_or(0, |(i, _)| i);
        let end = last_text.max(usize::from(screen.cursor_position().0)) + 1;
        screen.rows_formatted(0, width).take(end).skip(end.saturating_sub(usize::from(rows))).collect()
    }
}

/// A running observe child; killed on drop.
pub struct Observer {
    child: Child,
    pub mirror: Arc<Mutex<Mirror>>,
}

impl Observer {
    /// Observes `pane_id` at its full viewport size so the mirror can pick the bottom rows.
    pub fn spawn(pane_id: &str, cols: u16, rows: u16) -> io::Result<Self> {
        let herdr = std::env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string());
        let mut child = Command::new(herdr)
            .args(["terminal", "session", "observe", pane_id])
            .args(["--cols", &cols.to_string(), "--rows", &rows.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child.stdout.take().expect("stdout is piped");
        let mirror = Arc::new(Mutex::new(Mirror::new(cols, rows)));
        let shared = Arc::clone(&mirror);
        // Parses every frame as it arrives, which keeps the grid correct and drains stdout
        // before herdr's 30 s stall timeout drops the observer.
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let mut mirror = shared.lock().unwrap();
                mirror.apply(&line);
                if mirror.closed {
                    return;
                }
            }
            shared.lock().unwrap().closed = true;
        });
        Ok(Observer { child, mirror })
    }
}

impl Drop for Observer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frame(ansi: &str) -> String {
        let bytes = base64::engine::general_purpose::STANDARD.encode(ansi);
        json!({"type": "terminal.frame", "seq": 1, "encoding": "ansi", "width": 6,
               "height": 4, "full": true, "bytes": bytes})
        .to_string()
    }

    fn text(rows: &[Vec<u8>]) -> Vec<String> {
        rows.iter().map(|row| String::from_utf8_lossy(row).into_owned()).collect()
    }

    #[test]
    fn shows_bottom_rows_and_left_columns() {
        let mut mirror = Mirror::new(6, 4);
        mirror.apply(&frame("\x1b[2J\x1b[1;1Haaaaaa\x1b[2;1Hbbbbbb\x1b[3;1Hcccccc\x1b[4;1Hdddddd"));
        assert!(mirror.dirty);
        assert_eq!(text(&mirror.visible_rows(3, 2)), ["ccc", "ddd"]);
    }

    #[test]
    fn full_width_rows_carry_no_cross_row_cursor_moves() {
        let mut mirror = Mirror::new(6, 2);
        mirror.apply(&frame("\x1b[1;1Habcdef\x1b[2;1Hghij"));
        let rows = mirror.visible_rows(80, 2);
        assert_eq!(text(&rows), ["abcdef", "ghij"]);
        assert!(rows.iter().all(|row| !row.contains(&b'\r') && !row.contains(&b'\n')));
    }

    #[test]
    fn source_shorter_than_mirror_shows_every_row() {
        let mut mirror = Mirror::new(6, 2);
        mirror.apply(&frame("\x1b[1;1Hx\x1b[2;1Hy"));
        assert_eq!(text(&mirror.visible_rows(6, 10)), ["x", "y"]);
    }

    #[test]
    fn anchors_to_content_end_not_blank_rows_below_it() {
        let mut mirror = Mirror::new(6, 6);
        mirror.apply(&frame("\x1b[1;1Ha\x1b[2;1Hb\x1b[3;1H$ "));
        assert_eq!(text(&mirror.visible_rows(6, 2)), ["b", "$ "]);
    }

    #[test]
    fn anchor_includes_cursor_row_below_text() {
        let mut mirror = Mirror::new(6, 6);
        mirror.apply(&frame("\x1b[1;1Ha\x1b[2;1Hb\x1b[4;1H"));
        assert_eq!(text(&mirror.visible_rows(6, 2)), ["", ""]);
        assert_eq!(text(&mirror.visible_rows(6, 3)), ["b", "", ""]);
    }

    #[test]
    fn closed_record_ends_the_mirror() {
        let mut mirror = Mirror::new(6, 2);
        mirror.apply(r#"{"type":"terminal.closed","reason":"detached"}"#);
        assert!(mirror.closed);
    }
}
