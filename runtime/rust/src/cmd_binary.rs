//! `binary format` / `binary scan` (C ref `tclBinary.c`).
//!
//! Converts between Tcl values and binary byte strings via a format string of
//! `<type><count>` fields. Implemented type codes (both directions):
//!
//! - `a`/`A` — bytes, null-/space-padded
//! - `b`/`B` — binary-digit string (low-to-high / high-to-low within a byte)
//! - `h`/`H` — hex-digit string (low / high nibble first)
//! - `c` — 8-bit ints; `s`/`S`/`t`, `i`/`I`/`n`, `w`/`W`/`m` — 16/32/64-bit
//!   ints (little / big / native endian); `f`/`r`/`R` 32-bit, `d`/`q`/`Q`
//!   64-bit floats
//! - `x` skip/zero, `X` back up, `@` absolute position
//!
//! `count` is a number or `*` (all); native endian is little (the wasm/x86
//! target). Verified against tclsh 9.0. Unsigned scan modifiers and
//! `binary encode/decode` are deferred.
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{new_string, obj_bytes, Code, Interp};
use crate::obj::{self, TclObj};
use crate::parse::split_list;

/// Register `binary`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"binary", binary_cmd);
}

fn err(interp: &mut Interp, msg: &[u8]) -> Code {
    interp.set_error(msg)
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

fn binary_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        return wrong_args(interp, b"binary subcommand ?arg ...?");
    }
    match obj_bytes(argv[1]).as_slice() {
        b"format" => binary_format(interp, argv),
        b"scan" => binary_scan(interp, argv),
        other => {
            let mut m = b"unknown or ambiguous subcommand \"".to_vec();
            m.extend_from_slice(other);
            m.extend_from_slice(b"\": must be decode, encode, format, or scan");
            interp.set_error(&m)
        }
    }
}

/// Endianness of a multi-byte numeric field.
#[derive(Clone, Copy)]
enum End {
    Little,
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
            n = n.saturating_mul(10).saturating_add((c - b'0') as usize);
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

// -- format ----------------------------------------------------------------

/// Write `bytes` into `out` at `cur`, overwriting then extending; advance `cur`.
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

fn binary_format(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"binary format formatString ?arg ...?");
    }
    let fmt = obj_bytes(argv[2]);
    let args = &argv[3..];
    let mut ai = 0;
    let mut out: Vec<u8> = Vec::new();
    let mut cur = 0usize;
    let mut i = 0;

    macro_rules! next_arg {
        () => {{
            if ai >= args.len() {
                return err(interp, b"not enough arguments for all format specifiers");
            }
            let a = obj_bytes(args[ai]);
            ai += 1;
            a
        }};
    }

    while i < fmt.len() {
        let ty = fmt[i];
        if ty.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        i += 1;
        let count = parse_count(&fmt, &mut i);
        match ty {
            b'a' | b'A' => {
                let s = next_arg!();
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
                let s = next_arg!();
                let n = match count {
                    Count::Star => s.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                let bytes = pack_bits(&s, n, ty == b'B');
                put(&mut out, &mut cur, &bytes);
            }
            b'h' | b'H' => {
                let s = next_arg!();
                let n = match count {
                    Count::Star => s.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                let bytes = pack_hex(&s, n, ty == b'H');
                put(&mut out, &mut cur, &bytes);
            }
            b'c' | b's' | b'S' | b't' | b'i' | b'I' | b'n' | b'w' | b'W' | b'm' => {
                let (size, end) = int_kind(ty);
                let arg = next_arg!();
                let elems = match split_list(&arg) {
                    Ok(e) => e,
                    Err(e) => return err(interp, e.message()),
                };
                let n = match count {
                    Count::Star => elems.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                if elems.len() < n {
                    return err(interp, b"number of elements in list does not match count");
                }
                for e in &elems[..n] {
                    let Some(v) = parse_wide(e) else {
                        let mut m = b"expected integer but got \"".to_vec();
                        m.extend_from_slice(e);
                        m.push(b'"');
                        return err(interp, &m);
                    };
                    put(&mut out, &mut cur, &int_bytes(v, size, end));
                }
            }
            b'f' | b'r' | b'R' | b'd' | b'q' | b'Q' => {
                let (size, end) = float_kind(ty);
                let arg = next_arg!();
                let elems = match split_list(&arg) {
                    Ok(e) => e,
                    Err(e) => return err(interp, e.message()),
                };
                let n = match count {
                    Count::Star => elems.len(),
                    Count::Num(n) => n,
                    Count::None => 1,
                };
                if elems.len() < n {
                    return err(interp, b"number of elements in list does not match count");
                }
                for e in &elems[..n] {
                    let Some(v) = core::str::from_utf8(e)
                        .ok()
                        .and_then(|s| s.trim().parse::<f64>().ok())
                    else {
                        let mut m = b"expected floating-point number but got \"".to_vec();
                        m.extend_from_slice(e);
                        m.push(b'"');
                        return err(interp, &m);
                    };
                    put(&mut out, &mut cur, &float_bytes(v, size, end));
                }
            }
            b'x' => {
                let n = match count {
                    Count::Num(n) => n,
                    Count::Star | Count::None => 1,
                };
                let zeros = vec![0u8; n];
                put(&mut out, &mut cur, &zeros);
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
                let mut m = b"bad field specifier \"".to_vec();
                m.push(ty);
                m.push(b'"');
                return err(interp, &m);
            }
        }
    }
    interp.set_result_bytes(&out);
    Code::Ok
}

/// `(byte size, endianness)` for an integer type code.
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
fn int_bytes(v: i64, size: usize, end: End) -> Vec<u8> {
    let le = (v as u64).to_le_bytes();
    let mut b = le[..size].to_vec();
    if matches!(end, End::Big) {
        b.reverse();
    }
    b
}

/// `v` as an IEEE-754 float of `size` bytes in `end` order.
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
        let bit = matches!(s.get(k), Some(b'1'));
        if bit {
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

// -- scan ------------------------------------------------------------------

fn binary_scan(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 4 {
        return wrong_args(interp, b"binary scan string formatString ?varName ...?");
    }
    let data = obj_bytes(argv[2]);
    let fmt = obj_bytes(argv[3]);
    let vars = &argv[4..];
    let mut vi = 0;
    let mut cur = 0usize;
    let mut nconv = 0i64;
    let mut i = 0;

    // Assign `value` to the next varName; on failure, propagate the error.
    macro_rules! store {
        ($value:expr) => {{
            if vi >= vars.len() {
                return err(interp, b"not enough arguments for all format specifiers");
            }
            let name = obj_bytes(vars[vi]);
            vi += 1;
            let o = new_string(&$value);
            if interp.var_set(&name, o).is_err() {
                crate::interp::drop_fresh(o);
                let mut m = b"can't set \"".to_vec();
                m.extend_from_slice(&name);
                m.extend_from_slice(b"\": variable is array");
                return interp.set_error(&m);
            }
        }};
    }

    while i < fmt.len() {
        let ty = fmt[i];
        if ty.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        i += 1;
        let count = parse_count(&fmt, &mut i);
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
                    // `A` strips trailing spaces and nulls.
                    while matches!(s.last(), Some(b' ') | Some(0)) {
                        s.pop();
                    }
                }
                cur += n;
                store!(s);
                nconv += 1;
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
                let s = unpack_bits(&data[cur..], n, ty == b'B');
                cur += n.div_ceil(8);
                store!(s);
                nconv += 1;
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
                let s = unpack_hex(&data[cur..], n, ty == b'H');
                cur += n.div_ceil(2);
                store!(s);
                nconv += 1;
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
                    vals.push(v.to_string().into_bytes());
                }
                cur += n * size;
                let result = if matches!(count, Count::None) {
                    vals.into_iter().next().unwrap_or_default()
                } else {
                    join_list(&vals)
                };
                store!(result);
                nconv += 1;
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
                let result = if matches!(count, Count::None) {
                    vals.into_iter().next().unwrap_or_default()
                } else {
                    join_list(&vals)
                };
                store!(result);
                nconv += 1;
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
                let mut m = b"bad field specifier \"".to_vec();
                m.push(ty);
                m.push(b'"');
                return err(interp, &m);
            }
        }
    }
    interp.set_result(obj::new_wide_int_obj(nconv));
    Code::Ok
}

/// Read `size` bytes as a sign-extended integer in `end` order.
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
    // Sign-extend from `size` bytes.
    let bits = size * 8;
    if bits < 64 && (u >> (bits - 1)) & 1 == 1 {
        (u | (!0u64 << bits)) as i64
    } else {
        u as i64
    }
}

fn read_float(b: &[u8], size: usize, end: End) -> f64 {
    let mut buf = [0u8; 8];
    let n = size;
    if matches!(end, End::Big) {
        for k in 0..n {
            buf[k] = b[n - 1 - k];
        }
    } else {
        buf[..n].copy_from_slice(&b[..n]);
    }
    if size == 4 {
        f32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as f64
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

#[cfg(test)]
mod tests {
    use crate::counters;
    use crate::interp::{Code, Interp};

    fn leak_free(body: impl FnOnce(&mut Interp)) {
        counters::reset();
        {
            let mut interp = Interp::new();
            body(&mut interp);
        }
        assert_eq!(counters::finalize(), 0, "residual objs/bufs");
        assert_eq!(counters::double_free_count(), 0);
    }

    fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
        assert_eq!(
            i.eval_str(src),
            Code::Ok,
            "eval {:?} -> {:?}",
            String::from_utf8_lossy(src),
            String::from_utf8_lossy(&i.result_bytes())
        );
        i.result_bytes()
    }

    #[test]
    fn format_and_scan_roundtrip() {
        // Hex-dump helper round-trips through `binary scan H*`.
        leak_free(|i| {
            // format → hex (verified vs tclsh 9.0)
            i.eval_str(b"binary scan [binary format a5 foo] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"666f6f0000");
            i.eval_str(b"binary scan [binary format A5 foo] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"666f6f2020");
            i.eval_str(b"binary scan [binary format B8 01001101] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"4d");
            i.eval_str(b"binary scan [binary format b8 01001101] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"b2");
            i.eval_str(b"binary scan [binary format H2 4d] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"4d");
            i.eval_str(b"binary scan [binary format s 258] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"0201");
            i.eval_str(b"binary scan [binary format I 258] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"00000102");
            i.eval_str(b"binary scan [binary format c3 {1 2 3}] H* h; set ::h $h");
            assert_eq!(ok(i, b"set ::h"), b"010203");
            i.eval_str(b"unset ::h");
        });
    }

    #[test]
    fn scan_values_and_count() {
        leak_free(|i| {
            // single int (no count) → the value directly
            assert_eq!(
                ok(i, b"binary scan [binary format i 258] i v; set v"),
                b"258"
            );
            // count → a list
            assert_eq!(
                ok(i, b"binary scan [binary format c3 {1 2 3}] c3 v; set v"),
                b"1 2 3"
            );
            // signed 8-bit: 0xff → -1
            assert_eq!(
                ok(i, b"binary scan [binary format c 255] c v; set v"),
                b"-1"
            );
            // return value = number of conversions
            assert_eq!(ok(i, b"binary scan abc a2a1 x y"), b"2");
            assert_eq!(ok(i, b"set x"), b"ab");
            // `a*` takes the rest
            assert_eq!(ok(i, b"binary scan abcdef a* v; set v"), b"abcdef");
            i.eval_str(b"unset -nocomplain v x y");
        });
    }
}
