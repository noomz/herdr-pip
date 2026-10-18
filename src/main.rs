mod herdr;

use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use crossterm::terminal;
use herdr::Source;
use serde_json::{json, Map, Value};
use std::io::{self, Write};
use std::time::Duration;

const PAINT_TICK: Duration = Duration::from_millis(33);
/// Mirror scale in percent of the source size.
const DEFAULT_SCALE: u32 = 50;
const CLOSE_BUTTON: &str = "[x]";

fn main() {
    let result = match std::env::args().nth(1).as_deref() {
        Some("launch") => launch(),
        Some("peek") => peek(),
        _ => Err(io::Error::other("usage: herdr-pip launch|peek")),
    };
    if let Err(err) = result {
        eprintln!("herdr-pip: {err}");
        std::process::exit(1);
    }
}

/// Plugin action: open the peek popup, at the source's ratio size when `PIP_SOURCE`
/// names a pane, else at picker size.
fn launch() -> io::Result<()> {
    let Some(source_id) = std::env::var("PIP_SOURCE").ok().filter(|id| !id.is_empty()) else {
        return herdr::open_popup(json!("60%"), json!("50%"), Map::new());
    };
    let snap = herdr::snapshot()?;
    let source = find_source(&snap.sources, &source_id)?;
    let (width, height) = ratio_size(source, snap.area, DEFAULT_SCALE);
    let env = Map::from_iter([("PIP_SOURCE".to_string(), Value::from(source_id))]);
    herdr::open_popup(json!(width), json!(height), env)
}

fn find_source<'a>(sources: &'a [Source], pane_id: &str) -> io::Result<&'a Source> {
    sources
        .iter()
        .find(|s| s.pane_id == pane_id)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("pane {pane_id} not found")))
}

/// Outer popup size: the source scaled by `scale` percent plus border and top row,
/// clamped to at least 30x8 and at most 90%x85% of the pane surface.
fn ratio_size(source: &Source, area: (u16, u16), scale: u32) -> (u16, u16) {
    let fit = |cells: u16, chrome: u32, min: u32, max_pct: u32, area: u16| {
        let max = u32::from(area) * max_pct / 100;
        (u32::from(cells) * scale / 100 + chrome).min(max).max(min) as u16
    };
    (fit(source.cols, 2, 30, 90, area.0), fit(source.rows, 3, 8, 85, area.1))
}

/// The popup process: top row with source and `[x]`, read-only otherwise.
fn peek() -> io::Result<()> {
    let source = match std::env::var("PIP_SOURCE") {
        Ok(id) if !id.is_empty() => Some(find_source(&herdr::snapshot()?.sources, &id)?.clone()),
        _ => None,
    };
    let mut screen = Screen::enter()?;
    let mut notice = None;
    loop {
        screen.paint_top_row(&top_row_text(source.as_ref(), notice.as_deref()))?;
        if !event::poll(PAINT_TICK)? {
            continue;
        }
        let action = match event::read()? {
            Event::Key(key) => match key.code {
                KeyCode::Esc | KeyCode::Char('q') => Action::Close,
                KeyCode::Enter => Action::Jump,
                _ => Action::None,
            },
            Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
                let (cols, _) = terminal::size()?;
                if mouse.row == 0 && mouse.column >= cols.saturating_sub(CLOSE_BUTTON.len() as u16) {
                    Action::Close
                } else if mouse.row > 0 {
                    Action::Jump
                } else {
                    Action::None
                }
            }
            Event::Resize(..) => {
                screen.invalidate();
                Action::None
            }
            _ => Action::None,
        };
        match action {
            Action::None => {}
            Action::Close => return Ok(()),
            Action::Jump => match source.as_ref().map(|s| herdr::focus(&s.pane_id)) {
                Some(Ok(())) => return Ok(()),
                Some(Err(err)) => notice = Some(err.to_string()),
                None => {}
            },
        }
    }
}

enum Action {
    None,
    Close,
    Jump,
}

fn top_row_text(source: Option<&Source>, notice: Option<&str>) -> String {
    let mut text = source.map_or_else(|| "no source".to_string(), |s| s.name.clone());
    if let Some(status) = source.and_then(|s| s.status.as_deref()) {
        text = format!("{text} · {status}");
    }
    if let Some(notice) = notice {
        text = format!("{text} · {notice}");
    }
    text
}

/// Raw-mode alternate screen with SGR mouse reporting, restored on drop.
struct Screen {
    out: io::Stdout,
    drawn_top_row: Option<String>,
}

impl Screen {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[?1000h\x1b[?1006h\x1b[2J")?;
        out.flush()?;
        Ok(Screen { out, drawn_top_row: None })
    }

    fn invalidate(&mut self) {
        self.drawn_top_row = None;
    }

    /// Dim source text on the left, `[x]` on the right; redrawn only when it changes.
    fn paint_top_row(&mut self, text: &str) -> io::Result<()> {
        let (cols, _) = terminal::size()?;
        let line = top_row_line(text, cols as usize);
        if self.drawn_top_row.as_deref() == Some(line.as_str()) {
            return Ok(());
        }
        write!(self.out, "\x1b[1;1H\x1b[2m{line}\x1b[0m")?;
        self.out.flush()?;
        self.drawn_top_row = Some(line);
        Ok(())
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        let _ = self.out.write_all(b"\x1b[?1006l\x1b[?1000l\x1b[?25h\x1b[?1049l");
        let _ = self.out.flush();
        let _ = terminal::disable_raw_mode();
    }
}

/// `text` truncated and padded so the close button lands in the last columns.
fn top_row_line(text: &str, cols: usize) -> String {
    let room = cols.saturating_sub(CLOSE_BUTTON.len() + 1);
    let mut left: String = format!(" {text}").chars().take(room).collect();
    let pad = room.saturating_sub(left.chars().count());
    left.extend(std::iter::repeat_n(' ', pad + 1));
    if cols >= CLOSE_BUTTON.len() {
        left.push_str(CLOSE_BUTTON);
    }
    left
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(cols: u16, rows: u16) -> Source {
        Source { pane_id: "w1:p1".into(), name: "ws/tab/pane".into(), status: None, cols, rows }
    }

    #[test]
    fn ratio_size_scales_source_and_adds_chrome() {
        assert_eq!(ratio_size(&source(124, 63), (252, 65), 50), (64, 34));
    }

    #[test]
    fn ratio_size_clamps_to_minimum_and_area() {
        assert_eq!(ratio_size(&source(20, 6), (252, 65), 50), (30, 8));
        assert_eq!(ratio_size(&source(250, 63), (252, 65), 100), (226, 55));
    }

    #[test]
    fn top_row_puts_close_button_in_last_columns() {
        let line = top_row_line("Personal/dev/claude · working", 20);
        assert_eq!(line.chars().count(), 20);
        assert!(line.ends_with(" [x]"));
        assert!(line.starts_with(" Personal/dev/cl"));
    }
}
