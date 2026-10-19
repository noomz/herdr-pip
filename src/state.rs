//! Small settings remembered across peeks, one file each in `$XDG_STATE_HOME/herdr-pip/`
//! (falling back to `~/.local/state`).

use std::io;
use std::path::PathBuf;

pub fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(dir()?.join(name)).ok()
}

pub fn write(name: &str, value: &str) -> io::Result<()> {
    let dir = dir().ok_or_else(|| io::Error::other("no XDG_STATE_HOME or HOME"))?;
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(name), format!("{value}\n"))
}

fn dir() -> Option<PathBuf> {
    let state_home = std::env::var_os("XDG_STATE_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))?;
    Some(state_home.join("herdr-pip"))
}
