// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `binary` byte transforms — pure codecs (`encode`/`decode` hex/base64/
//! uuencode) and the `format` pack grammar, shared by both runtimes' `binary`
//! adapters.
//!
//! These are deliberately **value-model-free**: the two runtimes carry binary
//! data differently (the WASM runtime as the object's raw bytes; the bytecode
//! VM as a byte-array string, one `U+00xx` scalar per byte), so each adapter
//! converts its value to/from `&[u8]`/`Vec<u8>` in its own convention and the
//! codec in between is identical. Decoders report the offending byte + position
//! ([`DecodeError`]); the adapter renders the Tcl message and sets the
//! `TCL BINARY DECODE` error code, which differs by codec.

use tcl_syntax::list::split_list;

use crate::error::CmdError;

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

// encode

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

// decode

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

// format (the pack grammar)

/// Endianness of a multi-byte numeric field.
#[derive(Clone, Copy)]
pub enum End {
    /// Least-significant byte first.
    Little,
    /// Most-significant byte first.
    Big,
}

/// The count after a type code: an explicit number, `*` (all), or absent.
enum Count {
    Num(usize),
    Star,
    None,
}

/// Parse the optional count following a type char (advancing `i`).
fn parse_count(fmt: &[u8], i: &mut usize) -> Count {
    if fmt.get(*i) == Some(&b'*') {
        *i += 1;
        return Count::Star;
    }
    let start = *i;
    let mut n: usize = 0;
    while let Some(&c) = fmt.get(*i) {
        if c.is_ascii_digit() {
            n = n.saturating_mul(10).saturating_add(usize::from(c - b'0'));
            *i += 1;
        } else {
            break;
        }
    }
    if *i > start {
        Count::Num(n)
    } else {
        Count::None
    }
}

/// Parse a Tcl integer to a wide int — the shared grammar
/// ([`crate::sort::parse_wide`], the 9.0-first `tcl_syntax::number` integer
/// shape), narrowed by wrapping: an `i128` past `i64` wraps modulo 2⁶⁴,
/// matching C's `binary format` truncation of oversize values.
#[allow(clippy::cast_possible_truncation)] // i128 → i64 wrap matches C's `binary format`
fn parse_wide(b: &[u8]) -> Option<i64> {
    crate::sort::parse_wide(b).map(|v| v as i64)
}

/// Write `bytes` at the cursor `cur`, overwriting then extending `out`.
fn put(out: &mut Vec<u8>, cur: &mut usize, bytes: &[u8]) {
    for &b in bytes {
        if *cur < out.len() {
            out[*cur] = b;
        } else {
            out.push(b);
        }
        *cur += 1;
    }
}

/// `(byte size, endianness)` for an integer type code.
#[allow(clippy::match_same_arms)] // each Tcl code is listed explicitly for clarity
fn int_kind(ty: u8) -> (usize, End) {
    match ty {
        b'c' => (1, End::Little),
        b's' => (2, End::Little),
        b'S' => (2, End::Big),
        b't' => (2, End::Little), // native = little
        b'i' => (4, End::Little),
        b'I' => (4, End::Big),
        b'n' => (4, End::Little),
        b'w' => (8, End::Little),
        b'W' => (8, End::Big),
        _ => (8, End::Little), // m, native
    }
}

/// `(byte size, endianness)` for a float type code.
fn float_kind(ty: u8) -> (usize, End) {
    match ty {
        b'f' | b'r' => (4, End::Little),
        b'R' => (4, End::Big),
        b'd' | b'q' => (8, End::Little),
        _ => (8, End::Big), // Q
    }
}

/// `v`'s low `size` bytes in `end` order.
#[allow(clippy::cast_sign_loss)] // the bit pattern is what gets packed
fn int_bytes(v: i64, size: usize, end: End) -> Vec<u8> {
    let le = (v as u64).to_le_bytes();
    let mut b = le[..size].to_vec();
    if matches!(end, End::Big) {
        b.reverse();
    }
    b
}

/// `v` as an IEEE-754 float of `size` bytes in `end` order.
#[allow(clippy::cast_possible_truncation)] // f64 → f32 narrowing is the `f`/`r` field semantics
fn float_bytes(v: f64, size: usize, end: End) -> Vec<u8> {
    let mut b = if size == 4 {
        (v as f32).to_le_bytes().to_vec()
    } else {
        v.to_le_bytes().to_vec()
    };
    if matches!(end, End::Big) {
        b.reverse();
    }
    b
}

/// Pack `n` binary digits of `s` into ceil(n/8) bytes. `high_first` (`B`) fills
/// each byte MSB→LSB; `b` fills LSB→MSB. The consumed digits must all be `0`/`1`
/// (C errors `expected binary string but got "…" instead`); missing trailing
/// digits (when `n` exceeds the string) are zero-padded, not an error.
fn pack_bits(s: &[u8], n: usize, high_first: bool) -> Result<Vec<u8>, CmdError> {
    let mut out = vec![0u8; n.div_ceil(8)];
    for k in 0..n.min(s.len()) {
        match s[k] {
            b'0' => {}
            b'1' => {
                out[k / 8] |= if high_first {
                    0x80 >> (k % 8)
                } else {
                    1 << (k % 8)
                }
            }
            _ => return Err(expected_string("binary", s)),
        }
    }
    Ok(out)
}

/// Pack `n` hex digits of `s` into ceil(n/2) bytes. `high_first` (`H`) places
/// the first digit in the high nibble; `h` in the low nibble. The consumed
/// digits must all be hex (else C's `expected hexadecimal string but got "…"`).
fn pack_hex(s: &[u8], n: usize, high_first: bool) -> Result<Vec<u8>, CmdError> {
    let mut out = vec![0u8; n.div_ceil(2)];
    for k in 0..n.min(s.len()) {
        let Some(nib) = hex_nib(s[k]) else {
            return Err(expected_string("hexadecimal", s));
        };
        let high = (k % 2 == 0) == high_first;
        out[k / 2] |= if high { nib << 4 } else { nib };
    }
    Ok(out)
}

/// The error for a non-integer count-less integer field value: C reports a
/// multi-element value as `a list`, a single bad token verbatim.
fn int_value_error(arg: &[u8]) -> CmdError {
    let text = String::from_utf8_lossy(arg);
    if split_list(&text).is_ok_and(|e| e.len() != 1) {
        CmdError::new("expected integer but got a list".to_string())
    } else {
        CmdError::new(format!("expected integer but got \"{text}\""))
    }
}

/// C's `expected <kind> string but got "<arg>" instead` for an invalid `b`/`B`/
/// `h`/`H` `binary format` field value.
fn expected_string(kind: &str, arg: &[u8]) -> CmdError {
    CmdError::new(format!(
        "expected {kind} string but got \"{}\" instead",
        String::from_utf8_lossy(arg)
    ))
}

/// Take the next argument (each `args` element is one argument's byte rep),
/// erroring when the format string demands more than were supplied.
fn next_arg<'a>(args: &[&'a [u8]], ai: &mut usize) -> Result<&'a [u8], CmdError> {
    let a = args.get(*ai).copied().ok_or_else(|| {
        CmdError::new("not enough arguments for all format specifiers".to_string())
    })?;
    *ai += 1;
    Ok(a)
}

/// Allocate `n` bytes filled with `fill`, returning a catchable [`CmdError`]
/// instead of panicking on a capacity overflow. A `binary format` count may be
/// a saturated out-of-range value (`usize::MAX`); `vec![fill; n]` would abort
/// the process with "capacity overflow".
fn alloc_field(n: usize, fill: u8) -> Result<Vec<u8>, CmdError> {
    let mut v: Vec<u8> = Vec::new();
    v.try_reserve_exact(n)
        .map_err(|_| CmdError::new(format!("binary format: field size {n} is too large")))?;
    v.resize(n, fill);
    Ok(v)
}

/// `binary format formatString ?arg ...?` — pack the arguments (each given as
/// its byte representation) per the format string, returning the packed bytes.
/// The cursor model (`@`/`x`/`X`) and every type code are handled here; the
/// runtime adapter only converts its argument values to/from bytes.
#[allow(clippy::too_many_lines)] // one match arm per Tcl field type — a flat dispatch reads best
pub fn format(fmt: &[u8], args: &[&[u8]]) -> Result<Vec<u8>, CmdError> {
    let mut ai = 0usize;
    let mut out: Vec<u8> = Vec::new();
    let mut cur = 0usize;
    let mut i = 0usize;

    while i < fmt.len() {
        let ty = fmt[i];
        if ty.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        i += 1;
        let count = parse_count(fmt, &mut i);
        match ty {
            b'a' | b'A' => {
                let s = next_arg(args, &mut ai)?;
                let n = match count {
                    Count::Star => s.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                let pad = if ty == b'A' { b' ' } else { 0 };
                let mut field = alloc_field(n, pad)?;
                let take = s.len().min(n);
                field[..take].copy_from_slice(&s[..take]);
                put(&mut out, &mut cur, &field);
            }
            b'b' | b'B' => {
                let s = next_arg(args, &mut ai)?;
                let n = match count {
                    Count::Star => s.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                put(&mut out, &mut cur, &pack_bits(s, n, ty == b'B')?);
            }
            b'h' | b'H' => {
                let s = next_arg(args, &mut ai)?;
                let n = match count {
                    Count::Star => s.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                put(&mut out, &mut cur, &pack_hex(s, n, ty == b'H')?);
            }
            b'c' | b's' | b'S' | b't' | b'i' | b'I' | b'n' | b'w' | b'W' | b'm' => {
                let (size, end) = int_kind(ty);
                let arg = next_arg(args, &mut ai)?;
                if matches!(count, Count::None) {
                    // No count: the value is a *single* integer, not a list whose
                    // first element is taken (C: `binary format c {1 2}` errors).
                    let v = parse_wide(arg).ok_or_else(|| int_value_error(arg))?;
                    put(&mut out, &mut cur, &int_bytes(v, size, end));
                } else {
                    let elems = split_field(arg)?;
                    let n = field_count(&count, elems.len())?;
                    for e in &elems[..n] {
                        let v = parse_wide(e.as_bytes()).ok_or_else(|| {
                            CmdError::new(format!("expected integer but got \"{e}\""))
                        })?;
                        put(&mut out, &mut cur, &int_bytes(v, size, end));
                    }
                }
            }
            b'f' | b'r' | b'R' | b'd' | b'q' | b'Q' => {
                let (size, end) = float_kind(ty);
                let arg = next_arg(args, &mut ai)?;
                let elems = split_field(arg)?;
                let n = field_count(&count, elems.len())?;
                for e in &elems[..n] {
                    let v = e.trim().parse::<f64>().map_err(|_| {
                        CmdError::new(format!("expected floating-point number but got \"{e}\""))
                    })?;
                    put(&mut out, &mut cur, &float_bytes(v, size, end));
                }
            }
            b'x' => {
                let n = match count {
                    Count::Num(n) => n,
                    Count::Star | Count::None => 1,
                };
                put(&mut out, &mut cur, &alloc_field(n, 0)?);
            }
            b'X' => {
                let n = match count {
                    Count::Star => cur,
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                cur = cur.saturating_sub(n);
            }
            b'@' => {
                let n = match count {
                    Count::Star => out.len(),
                    Count::Num(n) => n,
                    Count::None => 0,
                };
                if n > out.len() {
                    out.resize(n, 0);
                }
                cur = n;
            }
            _ => {
                return Err(CmdError::new(format!(
                    "bad field specifier \"{}\"",
                    ty as char
                )));
            }
        }
    }
    Ok(out)
}

/// Split a numeric-field argument into list elements (the canonical Tcl list
/// message on a malformed value).
fn split_field(arg: &[u8]) -> Result<Vec<std::borrow::Cow<'_, str>>, CmdError> {
    let s = core::str::from_utf8(arg).map_err(|_| CmdError::new("invalid utf-8 in list"))?;
    split_list(s).map_err(|e| CmdError::new(e.message().to_string()))
}

/// Resolve a field's element count against the list length (erroring when the
/// list is too short for an explicit count).
fn field_count(count: &Count, available: usize) -> Result<usize, CmdError> {
    let n = match count {
        Count::Star => available,
        Count::Num(n) => *n,
        Count::None => 1,
    };
    if available < n {
        return Err(CmdError::new(
            "number of elements in list does not match count".to_string(),
        ));
    }
    Ok(n)
}

// scan (the unpack grammar)

/// Read `size` bytes as a sign-extended integer in `end` order.
#[allow(clippy::cast_possible_wrap)] // the unpacked bit pattern is the signed value
fn read_int(b: &[u8], size: usize, end: End) -> i64 {
    let mut buf = [0u8; 8];
    if matches!(end, End::Big) {
        for k in 0..size {
            buf[k] = b[size - 1 - k];
        }
    } else {
        buf[..size].copy_from_slice(&b[..size]);
    }
    let u = u64::from_le_bytes(buf);
    let bits = size * 8;
    if bits < 64 && (u >> (bits - 1)) & 1 == 1 {
        (u | (!0u64 << bits)) as i64
    } else {
        u as i64
    }
}

/// Read `size` bytes as an IEEE-754 float in `end` order.
fn read_float(b: &[u8], size: usize, end: End) -> f64 {
    let mut buf = [0u8; 8];
    if matches!(end, End::Big) {
        for k in 0..size {
            buf[k] = b[size - 1 - k];
        }
    } else {
        buf[..size].copy_from_slice(&b[..size]);
    }
    if size == 4 {
        f64::from(f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]))
    } else {
        f64::from_le_bytes(buf)
    }
}

/// Unpack `n` bits from `data` into a binary-digit string.
fn unpack_bits(data: &[u8], n: usize, high_first: bool) -> Vec<u8> {
    let mut s = Vec::with_capacity(n);
    for k in 0..n {
        let byte = data.get(k / 8).copied().unwrap_or(0);
        let pos = k % 8;
        let bit = if high_first {
            (byte >> (7 - pos)) & 1
        } else {
            (byte >> pos) & 1
        };
        s.push(b'0' + bit);
    }
    s
}

/// Unpack `n` hex digits from `data`.
fn unpack_hex(data: &[u8], n: usize, high_first: bool) -> Vec<u8> {
    let mut s = Vec::with_capacity(n);
    for k in 0..n {
        let byte = data.get(k / 2).copied().unwrap_or(0);
        let nib = if (k % 2 == 0) == high_first {
            byte >> 4
        } else {
            byte & 0x0f
        };
        s.push(b"0123456789abcdef"[nib as usize]);
    }
    s
}

/// Join scanned values into a Tcl list (each is a number/string with no special
/// chars, so a space-join is a valid list).
fn join_list(vals: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    for (k, v) in vals.iter().enumerate() {
        if k > 0 {
            out.push(b' ');
        }
        out.extend_from_slice(v);
    }
    out
}

/// `binary scan string formatString` — unpack `data` per the format string,
/// returning the values (each a byte string) to assign to the successive
/// `varName`s, in order. Scanning stops when `data` is exhausted; the caller
/// assigns the values (erroring if it runs out of variables) and returns the
/// conversion count.
#[allow(clippy::cast_sign_loss)] // the unsigned-mask path reinterprets the value's bits
#[allow(clippy::too_many_lines)] // one match arm per Tcl field type — a flat dispatch reads best
pub fn scan(data: &[u8], fmt: &[u8]) -> Result<Vec<Vec<u8>>, CmdError> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut cur = 0usize;
    let mut i = 0usize;
    while i < fmt.len() {
        let ty = fmt[i];
        if ty.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        i += 1;
        // A `u` after an integer type code requests unsigned interpretation.
        let unsigned = matches!(
            ty,
            b'c' | b's' | b'S' | b't' | b'i' | b'I' | b'n' | b'w' | b'W' | b'm'
        ) && fmt.get(i) == Some(&b'u');
        if unsigned {
            i += 1;
        }
        let count = parse_count(fmt, &mut i);
        match ty {
            b'a' | b'A' => {
                let n = match count {
                    Count::Star => data.len().saturating_sub(cur),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                // `n` may be `usize::MAX` (a saturated out-of-range count), so
                // `cur + n` must be checked — an unchecked add panics in debug
                // and wraps in release (`1 + usize::MAX → 0`), turning the
                // bounds check into `0 > len` (false) and slicing `data[1..0]`.
                // Not enough bytes → stop, like any short read.
                let Some(end) = cur.checked_add(n).filter(|&e| e <= data.len()) else {
                    break;
                };
                let mut s = data[cur..end].to_vec();
                if ty == b'A' {
                    while matches!(s.last(), Some(b' ' | 0)) {
                        s.pop();
                    }
                }
                cur = end;
                out.push(s);
            }
            b'b' | b'B' => {
                let n = match count {
                    Count::Star => data.len().saturating_sub(cur).saturating_mul(8),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                if cur + n.div_ceil(8) > data.len() {
                    break;
                }
                out.push(unpack_bits(&data[cur..], n, ty == b'B'));
                cur += n.div_ceil(8);
            }
            b'h' | b'H' => {
                let n = match count {
                    Count::Star => data.len().saturating_sub(cur).saturating_mul(2),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                if cur + n.div_ceil(2) > data.len() {
                    break;
                }
                out.push(unpack_hex(&data[cur..], n, ty == b'H'));
                cur += n.div_ceil(2);
            }
            b'c' | b's' | b'S' | b't' | b'i' | b'I' | b'n' | b'w' | b'W' | b'm' => {
                let (size, end) = int_kind(ty);
                let n = match count {
                    Count::Star => data.len().saturating_sub(cur) / size,
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                // `n` comes from the format string and `parse_count` saturates it
                // to `usize::MAX`, so `n * size` (and the cursor add) can overflow
                // usize and wrap *below* the bounds check, sneaking past it (and
                // then `Vec::with_capacity(n)` would try to allocate `usize::MAX`).
                // Compute the field end with checked arithmetic; an overflow means
                // the field cannot fit in `data`, so stop scanning like the normal
                // out-of-data path does.
                let fits = n
                    .checked_mul(size)
                    .and_then(|bytes| cur.checked_add(bytes))
                    .is_some_and(|field_end| field_end <= data.len());
                if !fits {
                    break;
                }
                let mut vals: Vec<Vec<u8>> = Vec::with_capacity(n);
                for k in 0..n {
                    let off = cur + k * size;
                    let v = read_int(&data[off..off + size], size, end);
                    if unsigned {
                        let mask: u64 = if size >= 8 {
                            u64::MAX
                        } else {
                            (1u64 << (size * 8)) - 1
                        };
                        vals.push(((v as u64) & mask).to_string().into_bytes());
                    } else {
                        vals.push(v.to_string().into_bytes());
                    }
                }
                cur += n * size;
                out.push(scan_field_result(&count, vals));
            }
            b'f' | b'r' | b'R' | b'd' | b'q' | b'Q' => {
                let (size, end) = float_kind(ty);
                let n = match count {
                    Count::Star => data.len().saturating_sub(cur) / size,
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                // As for the integer field above: a saturated `n` from the format
                // string makes `n * size` overflow usize and wrap past the bounds
                // check, so test "does the field fit" with checked arithmetic.
                let fits = n
                    .checked_mul(size)
                    .and_then(|bytes| cur.checked_add(bytes))
                    .is_some_and(|field_end| field_end <= data.len());
                if !fits {
                    break;
                }
                let mut vals: Vec<Vec<u8>> = Vec::with_capacity(n);
                for k in 0..n {
                    let off = cur + k * size;
                    let v = read_float(&data[off..off + size], size, end);
                    vals.push(tcl_syntax::number::format_double(v).into_bytes());
                }
                cur += n * size;
                out.push(scan_field_result(&count, vals));
            }
            b'x' => {
                let n = match count {
                    Count::Num(n) => n,
                    Count::Star | Count::None => 1,
                };
                cur = (cur + n).min(data.len());
            }
            b'X' => {
                let n = match count {
                    Count::Star => cur,
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                cur = cur.saturating_sub(n);
            }
            b'@' => {
                let n = match count {
                    Count::Star => data.len(),
                    Count::Num(n) => n,
                    Count::None => 0,
                };
                cur = n.min(data.len());
            }
            _ => {
                return Err(CmdError::new(format!(
                    "bad field specifier \"{}\"",
                    ty as char
                )));
            }
        }
    }
    Ok(out)
}

/// A scanned numeric field's stored value: the bare value for a countless field,
/// else the values as a Tcl list.
fn scan_field_result(count: &Count, vals: Vec<Vec<u8>>) -> Vec<u8> {
    if matches!(count, Count::None) {
        vals.into_iter().next().unwrap_or_default()
    } else {
        join_list(&vals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_a_field_with_huge_count_does_not_panic() {
        // An out-of-range count saturates to usize::MAX; the
        // `a`/`A` bounds check must not overflow `cur + n` (debug panic / release
        // wrap → OOB slice). `@1` advances cur to 1 first to exercise the add.
        let r = scan(b"xy", b"@1 a99999999999999999999999").expect("scan returns, no panic");
        // Not enough bytes for the huge field → the variable is unset (no
        // element scanned).
        assert!(r.is_empty(), "huge field should scan nothing: {r:?}");
        // A normal `a` field still works.
        assert_eq!(scan(b"xy", b"a2").unwrap(), vec![b"xy".to_vec()]);
    }

    #[test]
    fn format_a_field_with_huge_count_errors_not_panics() {
        // A huge field width must be a catchable CmdError, not a
        // process-aborting "capacity overflow".
        let err = format(b"a99999999999999999999999", &[b"x"]).unwrap_err();
        assert!(err.to_string().contains("too large"), "{err}");
        // The `x` (null pad) path is guarded too.
        assert!(format(b"x99999999999999999999999", &[]).is_err());
        // A normal field still formats.
        assert_eq!(format(b"a3", &[b"hi"]).unwrap(), b"hi\0");
    }

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

    #[test]
    fn format_scan_round_trip() {
        // `a3 c`: pack "foo" + a one-byte int 65 → "fooA"; scan reverses it.
        let packed = format(b"a3 c", &[b"foo", b"65"]).unwrap();
        assert_eq!(packed, b"fooA");
        assert_eq!(
            scan(b"fooA", b"a3 c").unwrap(),
            vec![b"foo".to_vec(), b"65".to_vec()]
        );
        // hex field round-trips.
        assert_eq!(format(b"H2", &[b"4d"]).unwrap(), vec![0x4d]);
        assert_eq!(scan(&[0x4d], b"H2").unwrap(), vec![b"4d".to_vec()]);
        // big-endian 16-bit (258 = 0x0102) and a `*` list count.
        assert_eq!(format(b"S", &[b"258"]).unwrap(), vec![0x01, 0x02]);
        assert_eq!(format(b"s*", &[b"1 2 3"]).unwrap(), vec![1, 0, 2, 0, 3, 0]);
        // errors: too few arguments, and a bad field specifier.
        assert!(format(b"c", &[]).is_err());
        assert!(format(b"Z", &[]).is_err());
        // scan stops when data runs out (returns the values it managed).
        assert_eq!(scan(b"ab", b"c c c").unwrap().len(), 2);
    }

    #[test]
    fn scan_huge_field_count_does_not_overflow() {
        // A field count from the format string is saturated to `usize::MAX`
        // by `parse_count`, so `n * size` used to overflow usize and wrap *under*
        // the bounds check (sneaking past it, then trying to allocate `usize::MAX`).
        // It must instead stop scanning, exactly like the normal out-of-data path.
        // Both an integer field (`w`, size 8) and a float field (`d`, size 8):
        let huge = b"w99999999999999999999"; // count saturates to usize::MAX
        assert!(scan(b"only-eight-bytes", huge).unwrap().is_empty());
        let huge_f = b"d99999999999999999999";
        assert!(scan(b"only-eight-bytes", huge_f).unwrap().is_empty());
        // A field that *does* fit still scans (regression guard).
        assert_eq!(scan(&[0x01, 0x02], b"s1").unwrap(), vec![b"513".to_vec()]);
    }
}
