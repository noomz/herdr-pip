//! Kitty graphics protocol: capability probe, double-buffered picture placement, letterbox fit.

use crate::raster::Picture;
use base64::Engine;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::time::{Duration, Instant};

/// The two image ids a peek alternates between: the new picture is placed before the old
/// one is deleted, because replacing one reused id flickers in Ghostty.
const IDS: [u32; 2] = [7, 8];
/// Base64 bytes per transmission chunk, the protocol's limit.
const CHUNK: usize = 4096;

/// Whether the terminal answers a kitty graphics query. Sends the query followed by a
/// primary device attributes request, whose reply ends the wait: a terminal without kitty
/// graphics answers only the second. Call in raw mode, before anything else reads stdin.
pub fn probe(timeout: Duration) -> io::Result<bool> {
    let mut out = io::stdout();
    out.write_all(b"\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c")?;
    out.flush()?;
    let fd = io::stdin().as_raw_fd();
    let deadline = Instant::now() + timeout;
    let mut reply = Vec::new();
    while !ends_with_device_attributes(&reply) {
        let left = deadline.saturating_duration_since(Instant::now());
        let mut poll = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
        // SAFETY: one valid pollfd, count 1.
        if left.is_zero() || unsafe { libc::poll(&mut poll, 1, left.as_millis() as i32) } <= 0 {
            break;
        }
        // Read the fd directly: `Stdin` buffers, and bytes held there would hide from `poll`.
        let mut buf = [0u8; 256];
        // SAFETY: writes at most `buf.len()` bytes into `buf`.
        let n = unsafe { libc::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
        if n <= 0 {
            break;
        }
        reply.extend_from_slice(&buf[..n as usize]);
    }
    Ok(contains(&reply, b"\x1b_Gi=31;OK"))
}

fn ends_with_device_attributes(reply: &[u8]) -> bool {
    reply.windows(3).position(|w| w == b"\x1b[?").is_some_and(|at| reply[at..].contains(&b'c'))
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// The picture currently on screen, so the next one replaces it without a gap.
#[derive(Default)]
pub struct Images {
    shown: Option<u32>,
}

impl Images {
    /// Places `picture` with its top-left at screen cell `at` (1-based column, row), scaled
    /// over `cells` (columns, rows), then deletes the previous picture.
    pub fn place(&mut self, out: &mut impl Write, picture: &Picture, at: (u16, u16), cells: (u16, u16)) -> io::Result<()> {
        let id = if self.shown == Some(IDS[0]) { IDS[1] } else { IDS[0] };
        let buf = transmission(picture, id, at, cells, self.shown)?;
        self.shown = Some(id);
        out.write_all(&buf)?;
        out.flush()
    }

    /// Deletes the picture on screen, if any.
    pub fn clear(&mut self, out: &mut impl Write) -> io::Result<()> {
        let Some(id) = self.shown.take() else { return Ok(()) };
        let mut buf = Vec::new();
        delete(&mut buf, id)?;
        out.write_all(&buf)?;
        out.flush()
    }
}

/// Moves the cursor to `at`, transmits and places `picture` as `id` in zlib-compressed
/// chunks without moving the cursor or asking for replies, then deletes `previous`.
fn transmission(picture: &Picture, id: u32, at: (u16, u16), cells: (u16, u16), previous: Option<u32>) -> io::Result<Vec<u8>> {
    let mut zlib = ZlibEncoder::new(Vec::new(), Compression::fast());
    zlib.write_all(&picture.rgb)?;
    let payload = base64::engine::general_purpose::STANDARD.encode(zlib.finish()?);
    let chunks: Vec<&[u8]> = payload.as_bytes().chunks(CHUNK).collect();
    let mut buf = Vec::with_capacity(payload.len() + chunks.len() * 16 + 64);
    write!(buf, "\x1b[{};{}H", at.1, at.0)?;
    for (i, chunk) in chunks.iter().enumerate() {
        let more = u8::from(i + 1 < chunks.len());
        if i == 0 {
            let (w, h, (c, r)) = (picture.width, picture.height, cells);
            write!(buf, "\x1b_Ga=T,f=24,o=z,s={w},v={h},i={id},c={c},r={r},C=1,q=2,m={more};")?;
        } else {
            write!(buf, "\x1b_Gm={more},q=2;")?;
        }
        buf.extend_from_slice(chunk);
        buf.extend_from_slice(b"\x1b\\");
    }
    if let Some(previous) = previous {
        delete(&mut buf, previous)?;
    }
    Ok(buf)
}

/// Deletes image `id` and frees its data.
fn delete(buf: &mut Vec<u8>, id: u32) -> io::Result<()> {
    write!(buf, "\x1b_Ga=d,d=I,i={id},q=2\x1b\\")
}

/// The largest area with `source`'s shape that fits in `area`, centered, both in cells:
/// (column offset, row offset, columns, rows).
pub fn letterbox(source: (u16, u16), area: (u16, u16)) -> (u16, u16, u16, u16) {
    let (sc, sr) = (u32::from(source.0.max(1)), u32::from(source.1.max(1)));
    let (ac, ar) = (u32::from(area.0), u32::from(area.1));
    let (cols, rows) = if sc * ar <= sr * ac { ((sc * ar / sr).max(1), ar) } else { (ac, (sr * ac / sc).max(1)) };
    let (cols, rows) = (cols.min(ac) as u16, rows.min(ar) as u16);
    ((area.0 - cols) / 2, (area.1 - rows) / 2, cols, rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> Picture {
        Picture { width: 2, height: 1, rgb: vec![0; 6] }
    }

    #[test]
    fn first_placement_uses_first_id_and_deletes_nothing() {
        let buf = String::from_utf8(transmission(&picture(), 7, (3, 2), (10, 5), None).unwrap()).unwrap();
        assert!(buf.starts_with("\x1b[2;3H\x1b_Ga=T,f=24,o=z,s=2,v=1,i=7,c=10,r=5,C=1,q=2,m=0;"));
        assert!(!buf.contains("a=d"));
    }

    #[test]
    fn placement_deletes_previous_after_placing_new() {
        let buf = String::from_utf8(transmission(&picture(), 8, (1, 2), (4, 4), Some(7)).unwrap()).unwrap();
        assert!(buf.find("i=8").unwrap() < buf.find("\x1b_Ga=d,d=I,i=7,q=2\x1b\\").unwrap());
    }

    #[test]
    fn large_payload_is_chunked() {
        // Noise does not compress, so the payload spans several chunks.
        let rgb = (0..30_000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
        let picture = Picture { width: 100, height: 100, rgb };
        let buf = String::from_utf8(transmission(&picture, 7, (1, 1), (1, 1), None).unwrap()).unwrap();
        let chunks: Vec<&str> = buf.split("\x1b_G").skip(1).collect();
        assert!(chunks.len() > 1);
        assert!(chunks[..chunks.len() - 1].iter().all(|c| c.contains("m=1")));
        assert!(chunks.last().unwrap().starts_with("m=0,q=2;"));
        assert!(chunks.iter().all(|c| c.split(';').nth(1).unwrap().len() <= CHUNK + 2));
    }

    #[test]
    fn images_alternate_ids() {
        let mut images = Images::default();
        let mut out = Vec::new();
        images.place(&mut out, &picture(), (1, 1), (1, 1)).unwrap();
        images.place(&mut out, &picture(), (1, 1), (1, 1)).unwrap();
        images.place(&mut out, &picture(), (1, 1), (1, 1)).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert_eq!(out.matches("a=T").count(), 3);
        assert_eq!(out.matches("i=7,c=").count(), 2);
        assert_eq!(out.matches("i=8,c=").count(), 1);
        images.clear(&mut Vec::new()).unwrap();
        assert!(images.shown.is_none());
    }

    #[test]
    fn letterbox_fits_exact_and_centers_the_rest() {
        assert_eq!(letterbox((80, 24), (40, 12)), (0, 0, 40, 12));
        // Area wider than the source: bars left and right.
        assert_eq!(letterbox((80, 24), (60, 12)), (10, 0, 40, 12));
        // Area taller than the source: bars above and below.
        assert_eq!(letterbox((80, 24), (40, 20)), (0, 4, 40, 12));
    }

    #[test]
    fn probe_reply_parsing() {
        assert!(ends_with_device_attributes(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;22c"));
        assert!(!ends_with_device_attributes(b"\x1b_Gi=31;OK\x1b\\\x1b[?62;2"));
        assert!(contains(b"\x1b_Gi=31;OK\x1b\\\x1b[?62c", b"\x1b_Gi=31;OK"));
    }
}
