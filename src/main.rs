mod herdr;
mod mirror;
mod picker;
mod scale;

use crossterm::event::{self, Event, KeyCode, MouseButton, MouseEventKind};
use crossterm::terminal;
use herdr::Source;
use mirror::{Mirror, Observer};
use picker::Picker;
use serde_json::{json, Map, Value};
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const PAINT_TICK: Duration = Duration::from_millis(33);
/// How often a float peek re-reads its source's size; the source can be resized beside it.
const SOURCE_CHECK: Duration = Duration::from_secs(2);
/// Picker size in percent of the pane surface.
const PICKER_WIDTH: &str = "60%";
const PICKER_HEIGHT: &str = "50%";
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

/// Plugin action: open the peek, at the source's ratio size when `PIP_SOURCE`
/// names a pane, else at picker size. A float peek hides while its source is focused.
fn launch() -> io::Result<()> {
    let Some(source_id) = std::env::var("PIP_SOURCE").ok().filter(|id| !id.is_empty()) else {
        return herdr::open_peek(None, json!(PICKER_WIDTH), json!(PICKER_HEIGHT), Map::new());
    };
    let snap = herdr::snapshot()?;
    let source = find_source(&snap.sources, &source_id)?;
    let (width, height) = ratio_size(source, snap.area, scale::load());
    let env = Map::from_iter([("PIP_SOURCE".to_string(), Value::from(source_id.as_str()))]);
    herdr::open_peek(Some(&source_id), json!(width), json!(height), env)
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

/// The peek process: top row with source or filter and `[x]`; below it the picker or a
/// read-only text mirror. `PIP_SOURCE` skips the picker.
fn peek() -> io::Result<()> {
    // A float stays open across a jump and switches between picker and mirror in place.
    let float_id = std::env::var("HERDR_FLOAT_ID").ok();
    let own_pane = std::env::var("HERDR_PANE_ID").ok();
    let mut view = match std::env::var("PIP_SOURCE") {
        Ok(id) if !id.is_empty() => View::mirror(find_source(&herdr::snapshot()?.sources, &id)?.clone())?,
        _ => View::Picker(Picker::new(&herdr::snapshot()?, own_pane.as_deref())),
    };
    let mut screen = Screen::enter()?;
    let mut notice = None;
    let mut source_checked = Instant::now();
    let mut scale = scale::load();
    loop {
        match &mut view {
            View::Mirror { source, observer } => {
                if observer.mirror.lock().unwrap().closed {
                    notice = Some("source closed".to_string());
                    view = show_picker(float_id.as_deref(), own_pane.as_deref())?;
                    screen.invalidate();
                    continue;
                }
                if source_checked.elapsed() >= SOURCE_CHECK {
                    source_checked = Instant::now();
                    if let Some(resized) = resized_source(source) {
                        *observer = Observer::spawn(&resized.pane_id, resized.cols, resized.rows)?;
                        *source = resized;
                        screen.invalidate();
                    }
                }
                screen.paint_top_row(&top_row_text(source, notice.as_deref()))?;
                screen.paint_mirror(&observer.mirror)?;
            }
            View::Picker(picker) => {
                screen.paint_top_row(&picker_top_row_text(&picker.filter, notice.as_deref()))?;
                let (cols, rows) = terminal::size()?;
                let body_rows = rows.saturating_sub(1);
                screen.paint_body(picker.lines(cols, body_rows), body_rows)?;
            }
        }
        if !event::poll(PAINT_TICK)? {
            continue;
        }
        let event = event::read()?;
        if let Event::Resize(..) = event {
            screen.invalidate();
            // Re-read the source size on the next tick.
            source_checked -= SOURCE_CHECK;
        }
        let action = match &mut view {
            View::Mirror { .. } => mirror_action(&event)?,
            View::Picker(picker) => picker_action(picker, &event)?,
        };
        match action {
            Action::None => {}
            Action::Close => return Ok(()),
            Action::Jump => {
                let View::Mirror { source, .. } = &view else { continue };
                match herdr::focus(&source.pane_id) {
                    Ok(()) if float_id.is_none() => return Ok(()),
                    Ok(()) => notice = None,
                    Err(err) => notice = Some(err.to_string()),
                }
            }
            Action::Scale(change) => {
                let View::Mirror { source, .. } = &view else { continue };
                let changed = scale::apply(scale, change);
                if changed == scale && change != scale::Change::Reset {
                    continue;
                }
                scale = changed;
                notice = scale::save(scale).err().map(|err| format!("scale not saved: {err}"));
                // A popup cannot resize; the new scale applies from the next peek.
                let Some(float_id) = &float_id else {
                    notice = notice.or_else(|| Some(format!("{scale}% · resize needs floats")));
                    continue;
                };
                let (width, height) = ratio_size(source, herdr::snapshot()?.area, scale);
                if let Err(err) = herdr::retarget_float(float_id, Some(&source.pane_id), json!(width), json!(height)) {
                    notice = Some(err.to_string());
                }
                screen.invalidate();
            }
            Action::Pick => {
                notice = None;
                view = show_picker(float_id.as_deref(), own_pane.as_deref())?;
                screen.invalidate();
            }
            Action::Peek(pane_id) => {
                let snap = herdr::snapshot()?;
                let Ok(source) = find_source(&snap.sources, &pane_id) else {
                    notice = Some("pane closed".to_string());
                    continue;
                };
                notice = None;
                if let Some(float_id) = &float_id {
                    let (width, height) = ratio_size(source, snap.area, scale);
                    let retargeted =
                        herdr::retarget_float(float_id, Some(&source.pane_id), json!(width), json!(height));
                    notice = retargeted.err().map(|err| err.to_string());
                }
                view = View::mirror(source.clone())?;
                source_checked = Instant::now();
                screen.invalidate();
            }
        }
    }
}

/// What the peek shows below its top row.
enum View {
    Picker(Picker),
    Mirror { source: Source, observer: Observer },
}

impl View {
    fn mirror(source: Source) -> io::Result<Self> {
        let observer = Observer::spawn(&source.pane_id, source.cols, source.rows)?;
        Ok(View::Mirror { source, observer })
    }
}

/// A fresh picker; a float also drops its hide rule and returns to picker size.
fn show_picker(float_id: Option<&str>, own_pane: Option<&str>) -> io::Result<View> {
    if let Some(float_id) = float_id {
        herdr::retarget_float(float_id, None, json!(PICKER_WIDTH), json!(PICKER_HEIGHT))?;
    }
    Ok(View::Picker(Picker::new(&herdr::snapshot()?, own_pane)))
}

enum Action {
    None,
    Close,
    Jump,
    /// Back to the picker.
    Pick,
    /// Show this pane in the mirror.
    Peek(String),
    /// Resize the mirror and remember the new scale.
    Scale(scale::Change),
}

fn is_close_click(column: u16, row: u16) -> io::Result<bool> {
    let (cols, _) = terminal::size()?;
    Ok(row == 0 && column >= cols.saturating_sub(CLOSE_BUTTON.len() as u16))
}

/// Mirror keys: Esc/`q` close, Enter or a click on the mirror jumps, `p`/Tab picks,
/// `+`/`-` step the scale and `=` resets it.
fn mirror_action(event: &Event) -> io::Result<Action> {
    Ok(match event {
        Event::Key(key) => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => Action::Close,
            KeyCode::Enter => Action::Jump,
            KeyCode::Char('p') | KeyCode::Tab => Action::Pick,
            KeyCode::Char('+') => Action::Scale(scale::Change::Larger),
            KeyCode::Char('-') => Action::Scale(scale::Change::Smaller),
            KeyCode::Char('=') => Action::Scale(scale::Change::Reset),
            _ => Action::None,
        },
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
            if is_close_click(mouse.column, mouse.row)? {
                Action::Close
            } else if mouse.row > 0 {
                Action::Jump
            } else {
                Action::None
            }
        }
        _ => Action::None,
    })
}

/// Picker keys: typing filters; with an empty filter `j`/`k` move and `q` closes.
/// Esc clears the filter, then closes. Enter or a click on a row peeks; the wheel moves.
fn picker_action(picker: &mut Picker, event: &Event) -> io::Result<Action> {
    let peek = |source: Option<&Source>| source.map_or(Action::None, |s| Action::Peek(s.pane_id.clone()));
    Ok(match event {
        Event::Key(key) => match key.code {
            KeyCode::Esc if !picker.filter.is_empty() => {
                picker.set_filter(String::new());
                Action::None
            }
            KeyCode::Esc => Action::Close,
            KeyCode::Char('q') if picker.filter.is_empty() => Action::Close,
            KeyCode::Char('j') if picker.filter.is_empty() => {
                picker.move_by(1);
                Action::None
            }
            KeyCode::Char('k') if picker.filter.is_empty() => {
                picker.move_by(-1);
                Action::None
            }
            KeyCode::Down => {
                picker.move_by(1);
                Action::None
            }
            KeyCode::Up => {
                picker.move_by(-1);
                Action::None
            }
            KeyCode::Char(c) => {
                picker.set_filter(format!("{}{c}", picker.filter));
                Action::None
            }
            KeyCode::Backspace => {
                let mut filter = picker.filter.clone();
                filter.pop();
                picker.set_filter(filter);
                Action::None
            }
            KeyCode::Enter => peek(picker.selected()),
            _ => Action::None,
        },
        Event::Mouse(mouse) => match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) if is_close_click(mouse.column, mouse.row)? => Action::Close,
            MouseEventKind::Down(MouseButton::Left) if mouse.row > 0 => {
                peek(picker.at_row(usize::from(mouse.row - 1)))
            }
            MouseEventKind::ScrollDown => {
                picker.move_by(1);
                Action::None
            }
            MouseEventKind::ScrollUp => {
                picker.move_by(-1);
                Action::None
            }
            _ => Action::None,
        },
        _ => Action::None,
    })
}

/// The source with its current size, when that differs from `source`'s.
fn resized_source(source: &Source) -> Option<Source> {
    let snap = herdr::snapshot().ok()?;
    let current = find_source(&snap.sources, &source.pane_id).ok()?;
    ((current.cols, current.rows) != (source.cols, source.rows)).then(|| current.clone())
}

fn top_row_text(source: &Source, notice: Option<&str>) -> String {
    let mut text = source.name.clone();
    if let Some(status) = source.status.as_deref() {
        text = format!("{text} · {status}");
    }
    with_notice(text, notice)
}

fn picker_top_row_text(filter: &str, notice: Option<&str>) -> String {
    let text = if filter.is_empty() { "pick a pane, type to filter".to_string() } else { format!("filter: {filter}") };
    with_notice(text, notice)
}

fn with_notice(text: String, notice: Option<&str>) -> String {
    match notice {
        Some(notice) => format!("{text} · {notice}"),
        None => text,
    }
}

/// Raw-mode alternate screen with SGR mouse reporting, restored on drop.
struct Screen {
    out: io::Stdout,
    drawn_top_row: Option<String>,
    /// Body rows as last drawn from screen row 2 down; `None` forces a full repaint.
    drawn_body: Option<Vec<Vec<u8>>>,
}

impl Screen {
    fn enter() -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        out.write_all(b"\x1b[?1049h\x1b[?25l\x1b[?1000h\x1b[?1006h\x1b[2J")?;
        out.flush()?;
        Ok(Screen { out, drawn_top_row: None, drawn_body: None })
    }

    fn invalidate(&mut self) {
        self.drawn_top_row = None;
        self.drawn_body = None;
    }

    /// Repaints the mirror when a frame arrived since the last paint.
    fn paint_mirror(&mut self, mirror: &Mutex<Mirror>) -> io::Result<()> {
        let mut mirror = mirror.lock().unwrap();
        if !mirror.dirty && self.drawn_body.is_some() {
            return Ok(());
        }
        mirror.dirty = false;
        let (cols, rows) = terminal::size()?;
        let body_rows = rows.saturating_sub(1);
        let lines = mirror.visible_rows(cols, body_rows);
        drop(mirror);
        self.paint_body(lines, body_rows)
    }

    /// Repaints the body rows that changed since the last paint, in one synchronized update.
    fn paint_body(&mut self, mut lines: Vec<Vec<u8>>, rows: u16) -> io::Result<()> {
        lines.resize(usize::from(rows), Vec::new());
        let drawn = self.drawn_body.take().unwrap_or_default();
        let mut changed = lines.iter().enumerate().filter(|(i, line)| drawn.get(*i) != Some(line)).peekable();
        if changed.peek().is_some() {
            let mut buf = b"\x1b[?2026h".to_vec();
            for (i, line) in changed {
                write!(buf, "\x1b[{};1H\x1b[0m\x1b[2K", i + 2)?;
                buf.extend_from_slice(line);
                buf.extend_from_slice(b"\x1b[0m");
            }
            buf.extend_from_slice(b"\x1b[?2026l");
            self.out.write_all(&buf)?;
            self.out.flush()?;
        }
        self.drawn_body = Some(lines);
        Ok(())
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
        Source { pane_id: "w1:p1".into(), name: "ws/tab/pane".into(), status: None, state_change_seq: 0, cols, rows }
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
