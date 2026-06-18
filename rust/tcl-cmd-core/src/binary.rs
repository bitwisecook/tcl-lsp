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

// -- format (the pack grammar) ----------------------------------------------

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

/// Parse a Tcl integer (optional sign + `0x`/`0o`/`0b`/decimal) to a wide int.
#[allow(clippy::cast_possible_truncation)] // i128 → i64 wrap matches C's `binary format`
fn parse_wide(b: &[u8]) -> Option<i64> {
    let s = core::str::from_utf8(b).ok()?.trim();
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let v: i128 = if let Some(h) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        i128::from_str_radix(h, 16).ok()?
    } else if let Some(o) = rest.strip_prefix("0o").or_else(|| rest.strip_prefix("0O")) {
        i128::from_str_radix(o, 8).ok()?
    } else if let Some(bb) = rest.strip_prefix("0b").or_else(|| rest.strip_prefix("0B")) {
        i128::from_str_radix(bb, 2).ok()?
    } else {
        rest.parse::<i128>().ok()?
    };
    Some((if neg { -v } else { v }) as i64)
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
/// each byte MSB→LSB; `b` fills LSB→MSB.
fn pack_bits(s: &[u8], n: usize, high_first: bool) -> Vec<u8> {
    let mut out = vec![0u8; n.div_ceil(8)];
    for k in 0..n {
        if matches!(s.get(k), Some(b'1')) {
            let byte = k / 8;
            let pos = k % 8;
            out[byte] |= if high_first { 0x80 >> pos } else { 1 << pos };
        }
    }
    out
}

/// Pack `n` hex digits of `s` into ceil(n/2) bytes. `high_first` (`H`) places
/// the first digit in the high nibble; `h` in the low nibble.
fn pack_hex(s: &[u8], n: usize, high_first: bool) -> Vec<u8> {
    let mut out = vec![0u8; n.div_ceil(2)];
    for k in 0..n {
        let nib = s.get(k).map_or(0, |&c| hex_val(c));
        let byte = k / 2;
        let high = (k % 2 == 0) == high_first;
        out[byte] |= if high { nib << 4 } else { nib };
    }
    out
}

fn hex_val(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    }
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
                let mut field = vec![pad; n];
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
                put(&mut out, &mut cur, &pack_bits(s, n, ty == b'B'));
            }
            b'h' | b'H' => {
                let s = next_arg(args, &mut ai)?;
                let n = match count {
                    Count::Star => s.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                put(&mut out, &mut cur, &pack_hex(s, n, ty == b'H'));
            }
            b'c' | b's' | b'S' | b't' | b'i' | b'I' | b'n' | b'w' | b'W' | b'm' => {
                let (size, end) = int_kind(ty);
                let arg = next_arg(args, &mut ai)?;
                let elems = split_field(arg)?;
                let n = field_count(&count, elems.len())?;
                for e in &elems[..n] {
                    let v = parse_wide(e.as_bytes()).ok_or_else(|| {
                        CmdError::new(format!("expected integer but got \"{e}\""))
                    })?;
                    put(&mut out, &mut cur, &int_bytes(v, size, end));
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
                put(&mut out, &mut cur, &vec![0u8; n]);
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

// -- scan (the unpack grammar) ----------------------------------------------

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
                if cur + n > data.len() {
                    break;
                }
                let mut s = data[cur..cur + n].to_vec();
                if ty == b'A' {
                    while matches!(s.last(), Some(b' ' | 0)) {
                        s.pop();
                    }
                }
                cur += n;
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
                if cur + n * size > data.len() {
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
                if cur + n * size > data.len() {
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
}
