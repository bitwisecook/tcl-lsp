//! `binary encode`/`decode` byte transforms — pure `&[u8] -> Vec<u8>` codecs
//! (hex, base64, uuencode), shared by both runtimes' `binary` adapters.
//!
//! These are deliberately **value-model-free**: the two runtimes carry binary
//! data differently (the WASM runtime as the object's raw bytes; the bytecode
//! VM as a byte-array string, one `U+00xx` scalar per byte), so each adapter
//! converts its value to/from `&[u8]`/`Vec<u8>` in its own convention and the
//! codec in between is identical. Decoders report the offending byte + position
//! ([`DecodeError`]); the adapter renders the Tcl message and sets the
//! `TCL BINARY DECODE` error code, which differs by codec.

/// A decode failure: the offending input byte and its position. The adapter
/// turns this into the codec-specific Tcl message (`invalid hexadecimal digit`
/// / `invalid base64 character`) plus the `TCL BINARY DECODE INVALID` code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeError {
    /// The offending byte.
    pub byte: u8,
    /// Its zero-based position in the input.
    pub pos: usize,
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[inline]
fn hex_digit(n: u8) -> u8 {
    if n < 10 { b'0' + n } else { b'a' + (n - 10) }
}

#[inline]
fn hex_nib(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[inline]
fn b64_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

// -- encode -----------------------------------------------------------------

/// `binary encode hex` — each byte as two lowercase hex digits.
#[must_use]
pub fn hex_encode(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 2);
    for &b in data {
        out.push(hex_digit(b >> 4));
        out.push(hex_digit(b & 0xf));
    }
    out
}

/// `binary encode base64` — RFC 4648 with `=` padding; `maxlen > 0` wraps each
/// line at `maxlen` output characters with `wrap`.
#[must_use]
pub fn base64_encode(data: &[u8], maxlen: usize, wrap: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut line = 0usize;
    let mut emit = |out: &mut Vec<u8>, c: u8| {
        if maxlen > 0 && line >= maxlen {
            out.extend_from_slice(wrap);
            line = 0;
        }
        out.push(c);
        line += 1;
    };
    for chunk in data.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        emit(&mut out, B64[(n >> 18) as usize & 63]);
        emit(&mut out, B64[(n >> 12) as usize & 63]);
        emit(
            &mut out,
            if chunk.len() > 1 {
                B64[(n >> 6) as usize & 63]
            } else {
                b'='
            },
        );
        emit(
            &mut out,
            if chunk.len() > 2 {
                B64[n as usize & 63]
            } else {
                b'='
            },
        );
    }
    out
}

/// `binary encode uuencode` — each line is a length byte (`count + 0x20`) then
/// groups of 4 chars per 3 data bytes (`value + 0x20`, `` ` `` for 0). `maxlen`
/// caps data bytes per line (default 45).
#[must_use]
#[allow(clippy::cast_possible_truncation)] // 6-bit groups / line counts fit a byte by construction
pub fn uu_encode(data: &[u8], maxlen: usize, wrap: &[u8]) -> Vec<u8> {
    let per_line = if maxlen >= 5 {
        (maxlen - 1) / 4 * 3
    } else {
        45
    };
    let mut out = Vec::new();
    let uc = |v: u8| -> u8 { if v == 0 { b'`' } else { 0x20 + v } };
    for line in data.chunks(per_line.max(1)) {
        out.push(uc(line.len() as u8));
        for chunk in line.chunks(3) {
            let b0 = u32::from(chunk[0]);
            let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
            let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
            let n = (b0 << 16) | (b1 << 8) | b2;
            out.push(uc((n >> 18) as u8 & 63));
            out.push(uc((n >> 12) as u8 & 63));
            out.push(uc((n >> 6) as u8 & 63));
            out.push(uc(n as u8 & 63));
        }
        out.extend_from_slice(wrap);
    }
    out
}

// -- decode -----------------------------------------------------------------

/// `binary decode hex` — pairs of hex digits to bytes; ASCII whitespace is
/// skipped, any other non-hex byte is a [`DecodeError`].
pub fn hex_decode(s: &[u8]) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::with_capacity(s.len() / 2);
    let mut hi: Option<u8> = None;
    for (pos, &c) in s.iter().enumerate() {
        let v = match hex_nib(c) {
            Some(v) => v,
            None if c.is_ascii_whitespace() => continue,
            None => return Err(DecodeError { byte: c, pos }),
        };
        match hi.take() {
            None => hi = Some(v),
            Some(h) => out.push((h << 4) | v),
        }
    }
    Ok(out)
}

/// `binary decode base64` — `strict` rejects whitespace other than CR/LF, any
/// character after padding, and any non-base64 byte; non-strict skips them.
#[allow(clippy::cast_possible_truncation)] // each `>> k` extracts one byte of the 24-bit group
pub fn base64_decode(s: &[u8], strict: bool) -> Result<Vec<u8>, DecodeError> {
    let mut out = Vec::new();
    let mut quad = [0u8; 4];
    let mut q = 0usize;
    let mut pads = 0usize;
    for (pos, &c) in s.iter().enumerate() {
        if c == b'=' {
            pads += 1;
            quad[q] = 0;
            q += 1;
        } else if let Some(v) = b64_val(c) {
            if pads > 0 && strict {
                return Err(DecodeError { byte: c, pos });
            }
            quad[q] = v;
            q += 1;
        } else if c.is_ascii_whitespace() {
            if strict && c != b'\n' && c != b'\r' {
                return Err(DecodeError { byte: c, pos });
            }
            continue;
        } else if strict {
            return Err(DecodeError { byte: c, pos });
        } else {
            continue;
        }
        if q == 4 {
            let n = (u32::from(quad[0]) << 18)
                | (u32::from(quad[1]) << 12)
                | (u32::from(quad[2]) << 6)
                | u32::from(quad[3]);
            out.push((n >> 16) as u8);
            if pads < 2 {
                out.push((n >> 8) as u8);
            }
            if pads < 1 {
                out.push(n as u8);
            }
            q = 0;
            pads = 0;
        }
    }
    Ok(out)
}

/// `binary decode uuencode` — each line's leading length byte gives the data
/// byte count; groups of 4 chars decode to 3 bytes. Lenient (no error).
#[must_use]
#[allow(clippy::cast_possible_truncation)] // each `>> k` extracts one byte of the 24-bit group
pub fn uu_decode(s: &[u8]) -> Vec<u8> {
    let dc = |c: u8| -> u8 { c.wrapping_sub(0x20) & 63 };
    let mut out = Vec::new();
    for line in s.split(|&c| c == b'\n') {
        let line: &[u8] = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        let count = dc(line[0]) as usize;
        let body = &line[1..];
        let mut produced = 0usize;
        for chunk in body.chunks(4) {
            if produced >= count {
                break;
            }
            let v: Vec<u8> = chunk.iter().map(|&c| dc(c)).collect();
            let n = (u32::from(v[0]) << 18)
                | (u32::from(*v.get(1).unwrap_or(&0)) << 12)
                | (u32::from(*v.get(2).unwrap_or(&0)) << 6)
                | u32::from(*v.get(3).unwrap_or(&0));
            for shift in [16, 8, 0] {
                if produced < count {
                    out.push((n >> shift) as u8);
                    produced += 1;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        assert_eq!(hex_encode(b"\x00\xff\x10"), b"00ff10");
        assert_eq!(hex_decode(b"00ff10").unwrap(), b"\x00\xff\x10");
        // whitespace is skipped
        assert_eq!(hex_decode(b"00 ff\n10").unwrap(), b"\x00\xff\x10");
        // a non-hex byte errors at its position
        assert_eq!(hex_decode(b"00g0"), Err(DecodeError { byte: b'g', pos: 2 }));
    }

    #[test]
    fn base64_round_trips() {
        assert_eq!(base64_encode(b"Man", 0, b"\n"), b"TWFu");
        assert_eq!(base64_encode(b"Ma", 0, b"\n"), b"TWE=");
        assert_eq!(base64_encode(b"M", 0, b"\n"), b"TQ==");
        assert_eq!(base64_decode(b"TWFu", false).unwrap(), b"Man");
        assert_eq!(base64_decode(b"TWE=", false).unwrap(), b"Ma");
        assert_eq!(base64_decode(b"TQ==", false).unwrap(), b"M");
        // strict rejects a stray non-base64 byte
        assert_eq!(
            base64_decode(b"TW!u", true),
            Err(DecodeError { byte: b'!', pos: 2 })
        );
        // non-strict skips it
        assert_eq!(base64_decode(b"TW!Fu", false).unwrap(), b"Man");
    }

    #[test]
    fn base64_wraps_at_maxlen() {
        assert_eq!(base64_encode(b"Manny", 4, b"\n"), b"TWFu\nbnk=");
    }

    #[test]
    fn uu_round_trips() {
        let enc = uu_encode(b"Cat", 0, b"\n");
        // length byte (3 → 0x23 '#') then one 4-char group, then wrap
        assert_eq!(&enc[..1], b"#");
        assert_eq!(uu_decode(&enc), b"Cat");
    }
}
