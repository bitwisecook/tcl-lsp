//! `binary format` / `binary scan` — the common subset.
//!
//! Bytes are carried in a string with the Tcl byte-array convention: byte `b`
//! is the scalar `U+00b`, so `string length` of the result is the byte count.
//! Supported specifiers: `a`/`A` (char strings), `c`/`s`/`S`/`i`/`I`/`w`/`W`
//! (8/16/32/64-bit integers, little/big-endian), `H`/`h` (hex, high/low nibble
//! first) and `x` (null padding).

use tcl_runtime_api::Completion;
use tcl_syntax::list::split_list;

use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("binary", cmd_binary);
}

/// Encode a byte slice as a Tcl byte-array string (one scalar per byte).
fn bytes_to_value(bytes: &[u8]) -> Value {
    Value::string(bytes.iter().map(|&b| char::from(b)).collect::<String>())
}

/// Decode a string back to bytes, taking the low 8 bits of each scalar.
#[allow(clippy::cast_possible_truncation)]
fn value_to_bytes(v: &Value) -> Vec<u8> {
    v.to_str().chars().map(|c| c as u32 as u8).collect()
}

/// A parsed `(specifier, count)` field; `count` is `None` for `*`.
struct Field {
    spec: char,
    count: Option<usize>,
    star: bool,
}

/// Parse a binary format string into its fields.
fn parse_fields(fmt: &str) -> Vec<Field> {
    let mut out = Vec::new();
    let mut it = fmt.chars().peekable();
    while let Some(&c) = it.peek() {
        if c.is_whitespace() {
            it.next();
            continue;
        }
        let spec = it.next().unwrap();
        let mut count = None;
        let mut star = false;
        if it.peek() == Some(&'*') {
            it.next();
            star = true;
        } else {
            let mut digits = String::new();
            while let Some(&d) = it.peek() {
                if d.is_ascii_digit() {
                    digits.push(d);
                    it.next();
                } else {
                    break;
                }
            }
            if !digits.is_empty() {
                count = Some(digits.parse().unwrap_or(0));
            }
        }
        out.push(Field { spec, count, star });
    }
    out
}

fn cmd_binary(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    let Some((sub, rest)) = args.split_first() else {
        return err("wrong # args: should be \"binary subcommand ?arg ...?\"");
    };
    match &*sub.to_str() {
        "format" => binary_format(rest),
        "scan" => binary_scan(vm, rest),
        other => err(format!("bad option \"{other}\": must be format or scan")),
    }
}

/// Width in bytes for an integer specifier.
fn int_width(spec: char) -> Option<usize> {
    match spec.to_ascii_lowercase() {
        'c' => Some(1),
        's' => Some(2),
        'i' => Some(4),
        'w' => Some(8),
        _ => None,
    }
}

/// Append `value` as a `width`-byte integer (little-endian for lowercase specs).
fn push_int(out: &mut Vec<u8>, value: i64, width: usize, big_endian: bool) {
    let bytes = value.to_le_bytes();
    if big_endian {
        for i in (0..width).rev() {
            out.push(bytes[i]);
        }
    } else {
        out.extend_from_slice(&bytes[..width]);
    }
}

/// Format an `a`/`A` char-string field.
fn format_chars(out: &mut Vec<u8>, f: &Field, arg: &Value) {
    let data = value_to_bytes(arg);
    let pad = if f.spec == 'A' { b' ' } else { 0 };
    let n = if f.star {
        data.len()
    } else {
        f.count.unwrap_or(1)
    };
    for i in 0..n {
        out.push(data.get(i).copied().unwrap_or(pad));
    }
}

/// Format an integer field (single value, or a list under a count/`*`).
/// Parse a Tcl integer literal (decimal or `0x`/`0o`/`0b` radix, optional sign),
/// returning `None` for malformed input. A radix literal is reinterpreted
/// bit-for-bit into `i64` (binary format only uses the low bytes).
#[allow(clippy::cast_possible_wrap)]
fn parse_tcl_int(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, body) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let mag = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok()? as i64
    } else if let Some(o) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
        u64::from_str_radix(o, 8).ok()? as i64
    } else if let Some(b) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
        u64::from_str_radix(b, 2).ok()? as i64
    } else {
        return body.parse::<i64>().ok().map(|v| if neg { -v } else { v });
    };
    Some(if neg { -mag } else { mag })
}

fn format_ints(out: &mut Vec<u8>, f: &Field, arg: &Value) -> Result<(), Completion<Value>> {
    let width = int_width(f.spec).unwrap_or(1);
    let big = f.spec.is_ascii_uppercase();
    if f.count.is_none() && !f.star {
        let v = arg.as_int().map_err(|e| err(e.message))?;
        push_int(out, v, width, big);
        return Ok(());
    }
    let list = split_list(&arg.to_str())
        .map(|l| l.iter().map(ToString::to_string).collect::<Vec<_>>())
        .unwrap_or_default();
    let n = if f.star {
        list.len()
    } else {
        f.count.unwrap_or(0)
    };
    for i in 0..n {
        let s = list.get(i).map_or("", String::as_str);
        let v = parse_tcl_int(s).ok_or_else(|| err(format!("expected integer but got \"{s}\"")))?;
        push_int(out, v, width, big);
    }
    Ok(())
}

/// Format an `H`/`h` hex field.
fn format_hex(out: &mut Vec<u8>, f: &Field, arg: &Value) {
    let hex: Vec<u8> = arg
        .to_str()
        .chars()
        .filter_map(|c| c.to_digit(16).map(|d| u8::try_from(d).unwrap_or(0)))
        .collect();
    let nibbles = if f.star {
        hex.len()
    } else {
        f.count.unwrap_or(1)
    };
    let high_first = f.spec == 'H';
    let mut i = 0;
    while i < nibbles {
        let hi = hex.get(i).copied().unwrap_or(0);
        let lo = hex.get(i + 1).copied().unwrap_or(0);
        out.push(if high_first {
            (hi << 4) | lo
        } else {
            (lo << 4) | hi
        });
        i += 2;
    }
}

fn binary_format(rest: &[Value]) -> Completion<Value> {
    let Some((fmt, args)) = rest.split_first() else {
        return err("wrong # args: should be \"binary format formatString ?arg ...?\"");
    };
    let fields = parse_fields(&fmt.to_str());
    let mut out: Vec<u8> = Vec::new();
    let mut ai = 0;
    for f in &fields {
        if f.spec == 'x' {
            out.extend(std::iter::repeat_n(0u8, f.count.unwrap_or(1)));
            continue;
        }
        let Some(arg) = args.get(ai) else {
            return err("not enough arguments for all format specifiers");
        };
        ai += 1;
        match f.spec {
            'a' | 'A' => format_chars(&mut out, f, arg),
            'c' | 's' | 'S' | 'i' | 'I' | 'w' | 'W' => {
                if let Err(e) = format_ints(&mut out, f, arg) {
                    return e;
                }
            }
            'H' | 'h' => format_hex(&mut out, f, arg),
            other => return err(format!("bad field specifier \"{other}\"")),
        }
    }
    ok(bytes_to_value(&out))
}

fn read_int(bytes: &[u8], width: usize, big: bool, signed: bool) -> i64 {
    let mut buf = [0u8; 8];
    if big {
        for i in 0..width {
            buf[width - 1 - i] = bytes[i];
        }
    } else {
        buf[..width].copy_from_slice(&bytes[..width]);
    }
    let raw = i64::from_le_bytes(buf);
    if signed && width < 8 {
        let bits = width * 8;
        let sign = 1i64 << (bits - 1);
        if raw & sign != 0 {
            return raw - (1i64 << bits);
        }
    }
    raw
}

/// Read an `a`/`A` field → `(value, new_pos)`, or `None` if too few bytes.
fn scan_chars(bytes: &[u8], pos: usize, f: &Field) -> Option<(Value, usize)> {
    let n = if f.star {
        bytes.len().saturating_sub(pos)
    } else {
        f.count.unwrap_or(1)
    };
    if pos + n > bytes.len() {
        return None;
    }
    let mut slice = bytes[pos..pos + n].to_vec();
    if f.spec == 'A' {
        while matches!(slice.last(), Some(&(b' ' | 0))) {
            slice.pop();
        }
    }
    Some((bytes_to_value(&slice), pos + n))
}

/// Read an integer field (single value, or a list under a count/`*`).
fn scan_ints(bytes: &[u8], pos: usize, f: &Field) -> Option<(Value, usize)> {
    let width = int_width(f.spec).unwrap_or(1);
    let big = f.spec.is_ascii_uppercase();
    if f.count.is_none() && !f.star {
        if pos + width > bytes.len() {
            return None;
        }
        return Some((
            Value::int(read_int(&bytes[pos..], width, big, true)),
            pos + width,
        ));
    }
    let n = if f.star {
        (bytes.len() - pos) / width
    } else {
        f.count.unwrap_or(0)
    };
    let mut items = Vec::new();
    let mut p = pos;
    for _ in 0..n {
        if p + width > bytes.len() {
            break;
        }
        items.push(Value::int(read_int(&bytes[p..], width, big, true)));
        p += width;
    }
    Some((Value::list(items), p))
}

/// Read an `H`/`h` hex field → `(value, new_pos)`.
fn scan_hex(bytes: &[u8], pos: usize, f: &Field) -> (Value, usize) {
    let n = if f.star {
        (bytes.len() - pos) * 2
    } else {
        f.count.unwrap_or(1)
    };
    let high_first = f.spec == 'H';
    let mut s = String::new();
    for i in 0..n {
        let Some(&byte) = bytes.get(pos + i / 2) else {
            break;
        };
        let nib = if (i % 2 == 0) == high_first {
            byte >> 4
        } else {
            byte & 0xf
        };
        s.push(char::from_digit(u32::from(nib), 16).unwrap_or('0'));
    }
    (Value::string(s), (pos + n.div_ceil(2)).min(bytes.len()))
}

fn binary_scan(vm: &mut Vm, rest: &[Value]) -> Completion<Value> {
    let Some((data, more)) = rest.split_first() else {
        return err("wrong # args: should be \"binary scan value formatString ?varName ...?\"");
    };
    let Some((fmt, vars)) = more.split_first() else {
        return err("wrong # args: should be \"binary scan value formatString ?varName ...?\"");
    };
    let fields = parse_fields(&fmt.to_str());
    let bytes = value_to_bytes(data);
    let mut pos = 0usize;
    let mut vi = 0usize;
    let mut count = 0i64;
    for f in &fields {
        let (val, new_pos) = match f.spec {
            'a' | 'A' => match scan_chars(&bytes, pos, f) {
                Some(r) => r,
                None => break,
            },
            'c' | 's' | 'S' | 'i' | 'I' | 'w' | 'W' => match scan_ints(&bytes, pos, f) {
                Some(r) => r,
                None => break,
            },
            'H' | 'h' => scan_hex(&bytes, pos, f),
            'x' => {
                pos = (pos + f.count.unwrap_or(1)).min(bytes.len());
                continue;
            }
            other => return err(format!("bad field specifier \"{other}\"")),
        };
        pos = new_pos;
        let Some(name) = vars.get(vi) else {
            return err("not enough arguments for all format specifiers");
        };
        vi += 1;
        if let Err(e) = vm.set_var(&name.to_str(), val) {
            return e;
        }
        count += 1;
    }
    ok(Value::int(count))
}
