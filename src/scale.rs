//! Mirror scale in percent of the source size: one global value, remembered across peeks
//! in `$XDG_STATE_HOME/herdr-pip/scale`.

use std::io;
use std::path::PathBuf;

pub const DEFAULT: u32 = 50;
const MIN: u32 = 20;
const MAX: u32 = 100;
const STEP: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    Larger,
    Smaller,
    Reset,
}

pub fn apply(scale: u32, change: Change) -> u32 {
    match change {
        Change::Larger => (scale + STEP).min(MAX),
        Change::Smaller => scale.saturating_sub(STEP).max(MIN),
        Change::Reset => DEFAULT,
    }
}

/// The remembered scale, or the default when none is stored or it does not parse.
pub fn load() -> u32 {
    state_file()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map_or(DEFAULT, |text| parse(&text))
}

pub fn save(scale: u32) -> io::Result<()> {
    let path = state_file().ok_or_else(|| io::Error::other("no XDG_STATE_HOME or HOME"))?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, format!("{scale}\n"))
}

fn parse(text: &str) -> u32 {
    text.trim().parse().map_or(DEFAULT, |scale: u32| scale.clamp(MIN, MAX))
}

fn state_file() -> Option<PathBuf> {
    let state_home = std::env::var_os("XDG_STATE_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(state_home.join("herdr-pip/scale"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_by_ten_within_limits_and_resets_to_default() {
        assert_eq!(apply(50, Change::Larger), 60);
        assert_eq!(apply(95, Change::Larger), 100);
        assert_eq!(apply(50, Change::Smaller), 40);
        assert_eq!(apply(25, Change::Smaller), 20);
        assert_eq!(apply(80, Change::Reset), 50);
    }

    #[test]
    fn parse_clamps_and_falls_back_to_default() {
        assert_eq!(parse("70\n"), 70);
        assert_eq!(parse("5"), 20);
        assert_eq!(parse("500"), 100);
        assert_eq!(parse("big"), 50);
    }
}
