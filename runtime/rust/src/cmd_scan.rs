//! `scan` — parse a string under a format (the `sscanf` analogue), per
//! `tmp/tcl9.0.3/generic/tclScan.c` (`Tcl_ScanObjCmd`). With `varName`s it
//! assigns and returns the conversion count (`-1` on EOF before any
//! conversion); with none it works *inline*, returning the scanned values as a
//! list.
//!
//! Conversions: `%d`/`%i`/`%u`/`%o`/`%x`/`%b`/`%c`/`%s`/`%e`/`%f`/`%g`/`%[...]`/
//! `%n`/`%%`, with `*` (suppress), a field width, and ignored size modifiers
//! (`h`/`l`/`L`/…). Operates on Unicode code points (so `%c` and widths count
//! characters).
//!
//! See `list.rs` for the module-level `not_unsafe_ptr_arg_deref` rationale.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

use crate::interp::{drop_fresh, obj_bytes, Code, Interp};
use crate::obj::{new_string_bytes, new_wide_int_obj, TclObj};

/// Register `scan`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"scan", scan_cmd);
}

fn wrong_args(interp: &mut Interp, usage: &[u8]) -> Code {
    let mut m = b"wrong # args: should be \"".to_vec();
    m.extend_from_slice(usage);
    m.push(b'"');
    interp.set_error(&m)
}

/// One scanned value: a string (already formatted) to assign / collect.
type Scanned = Vec<u8>;

fn scan_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 3 {
        return wrong_args(interp, b"scan string format ?varName ...?");
    }
    let input: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[1]))
        .chars()
        .collect();
    let fmt: Vec<char> = String::from_utf8_lossy(&obj_bytes(argv[2]))
        .chars()
        .collect();
    let vars = &argv[3..];
    let inline = vars.is_empty();

    let mut ii = 0; // input cursor (chars)
    let mut fi = 0; // format cursor (chars)
                    // Per non-suppressed conversion: the scanned value, or None if it failed
                    // (inline mode pads failures with an empty string).
    let mut values: Vec<Option<Scanned>> = Vec::new();
    let mut nconv = 0; // successful, non-suppressed, non-`%n` conversions
    let mut eof_before_conv = false;

    while fi < fmt.len() {
        let fc = fmt[fi];
        if fc.is_whitespace() {
            while ii < input.len() && input[ii].is_whitespace() {
                ii += 1;
            }
            fi += 1;
            continue;
        }
        if fc != '%' {
            if ii < input.len() && input[ii] == fc {
                ii += 1;
                fi += 1;
                continue;
            }
            break; // literal mismatch
        }
        // A conversion specifier.
        fi += 1;
        if fi >= fmt.len() {
            break;
        }
        if fmt[fi] == '%' {
            if ii < input.len() && input[ii] == '%' {
                ii += 1;
                fi += 1;
                continue;
            }
            break;
        }
        let mut suppress = false;
        if fmt[fi] == '*' {
            suppress = true;
            fi += 1;
        }
        let mut width = 0usize;
        let mut has_width = false;
        while fi < fmt.len() && fmt[fi].is_ascii_digit() {
            has_width = true;
            width = width * 10 + (fmt[fi] as usize - '0' as usize);
            fi += 1;
        }
        // Ignore size modifiers (we scan into i64/f64).
        while fi < fmt.len() && matches!(fmt[fi], 'h' | 'l' | 'L' | 'q' | 'j' | 'z' | 't') {
            fi += 1;
        }
        if fi >= fmt.len() {
            break;
        }
        let conv = fmt[fi];
        fi += 1;

        // `%n` reports the characters consumed so far; it doesn't consume input
        // or count as a conversion.
        if conv == 'n' {
            if !suppress {
                push_value(&mut values, Some(ii.to_string().into_bytes()));
            }
            continue;
        }

        // Most conversions skip leading whitespace; `%c` and `%[` do not.
        if !matches!(conv, 'c' | '[') {
            while ii < input.len() && input[ii].is_whitespace() {
                ii += 1;
            }
        }
        if ii >= input.len() && nconv == 0 {
            eof_before_conv = true;
        }

        let field_max = if has_width { width } else { usize::MAX };
        let scanned = scan_one(&input, &mut ii, conv, field_max, &fmt, &mut fi);
        match scanned {
            Some(v) => {
                if !suppress {
                    push_value(&mut values, Some(v));
                    nconv += 1;
                }
            }
            None => {
                // Conversion failed: stop. Inline mode keeps a hole.
                if !suppress {
                    push_value(&mut values, None);
                }
                break;
            }
        }
    }

    if inline {
        // The list of scanned values; trailing unmatched conversions render as
        // empty strings (Tcl), but an outright EOF-before-anything is empty.
        if values.is_empty() {
            interp.set_result_bytes(b"");
            return Code::Ok;
        }
        let objs: Vec<*mut TclObj> = values
            .iter()
            .map(|v| new_string_bytes(v.as_deref().unwrap_or(b"")))
            .collect();
        interp.set_result(crate::list::new_list_obj(&objs));
        return Code::Ok;
    }

    // Variable mode: assign each successful value to its var, return the count
    // (or -1 if EOF hit before any conversion matched).
    if nconv == 0 && eof_before_conv {
        interp.set_result(new_wide_int_obj(-1));
        return Code::Ok;
    }
    let mut assigned = 0;
    for (v, &var) in values.iter().zip(vars.iter()) {
        let Some(bytes) = v else { break };
        let name = obj_bytes(var);
        let o = new_string_bytes(bytes);
        if interp.var_set(&name, o).is_err() {
            drop_fresh(o);
            let mut m = b"can't set \"".to_vec();
            m.extend_from_slice(&name);
            m.extend_from_slice(b"\"");
            return interp.set_error(&m);
        }
        assigned += 1;
    }
    interp.set_result(new_wide_int_obj(assigned as i64));
    Code::Ok
}

fn push_value(values: &mut Vec<Option<Scanned>>, v: Option<Scanned>) {
    values.push(v);
}

/// Scan one field for `conv`, advancing `ii`. Returns the formatted value, or
/// `None` if nothing valid was read. `fi` is advanced past a `%[...]` set.
fn scan_one(
    input: &[char],
    ii: &mut usize,
    conv: char,
    width: usize,
    fmt: &[char],
    fi: &mut usize,
) -> Option<Scanned> {
    match conv {
        'c' => {
            // One character → its code point. (No width, no whitespace skip.)
            if *ii >= input.len() {
                return None;
            }
            let cp = input[*ii] as u32;
            *ii += 1;
            Some(cp.to_string().into_bytes())
        }
        'd' | 'u' => scan_int(input, ii, width, 10, true),
        'i' => scan_int_auto(input, ii, width),
        'o' => scan_int(input, ii, width, 8, false),
        'x' | 'X' => scan_int(input, ii, width, 16, false),
        'b' => scan_int(input, ii, width, 2, false),
        's' => {
            let start = *ii;
            let mut n = 0;
            while *ii < input.len() && !input[*ii].is_whitespace() && n < width {
                *ii += 1;
                n += 1;
            }
            if *ii == start {
                return None;
            }
            Some(chars_to_bytes(&input[start..*ii]))
        }
        'e' | 'E' | 'f' | 'g' | 'G' => scan_float(input, ii, width),
        '[' => scan_set(input, ii, width, fmt, fi),
        _ => None, // unknown conversion
    }
}

/// `%d`/`%o`/`%x`/`%b`/`%u`: an optionally-signed integer in `radix`.
fn scan_int(
    input: &[char],
    ii: &mut usize,
    width: usize,
    radix: u32,
    signed: bool,
) -> Option<Scanned> {
    let start = *ii;
    let mut s = String::new();
    let mut n = 0;
    if signed && *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        s.push(input[*ii]);
        *ii += 1;
        n += 1;
    }
    // Allow a `0x`/`0b` prefix for hex/binary.
    if (radix == 16 || radix == 2) && *ii + 1 < input.len() && input[*ii] == '0' {
        let p = input[*ii + 1].to_ascii_lowercase();
        if (radix == 16 && p == 'x') || (radix == 2 && p == 'b') {
            *ii += 2;
            n += 2;
        }
    }
    let digits_start = *ii;
    while *ii < input.len() && n < width && input[*ii].is_digit(radix) {
        s.push(input[*ii]);
        *ii += 1;
        n += 1;
    }
    if *ii == digits_start {
        *ii = start;
        return None;
    }
    let val = i64::from_str_radix(&s, radix).ok()?;
    Some(val.to_string().into_bytes())
}

/// `%i`: an integer with C-style base detection (`0x`→16, `0`→8, else 10).
fn scan_int_auto(input: &[char], ii: &mut usize, width: usize) -> Option<Scanned> {
    let save = *ii;
    let mut sign = 1i64;
    let mut n = 0;
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        if input[*ii] == '-' {
            sign = -1;
        }
        *ii += 1;
        n += 1;
    }
    let (radix, _consumed) = if *ii + 1 < input.len() && input[*ii] == '0' {
        match input[*ii + 1].to_ascii_lowercase() {
            'x' => {
                *ii += 2;
                n += 2;
                (16u32, 2)
            }
            'b' => {
                *ii += 2;
                n += 2;
                (2u32, 2)
            }
            _ => (8u32, 0),
        }
    } else {
        (10u32, 0)
    };
    let ds = *ii;
    let mut s = String::new();
    while *ii < input.len() && n < width && input[*ii].is_digit(radix) {
        s.push(input[*ii]);
        *ii += 1;
        n += 1;
    }
    if *ii == ds {
        *ii = save;
        return None;
    }
    let val = i64::from_str_radix(&s, radix).ok()? * sign;
    Some(val.to_string().into_bytes())
}

/// `%e`/`%f`/`%g`: a floating-point number.
fn scan_float(input: &[char], ii: &mut usize, width: usize) -> Option<Scanned> {
    let start = *ii;
    let mut s = String::new();
    let mut n = 0;
    let take = |c: char, ii: &mut usize, s: &mut String, n: &mut usize| {
        s.push(c);
        *ii += 1;
        *n += 1;
    };
    if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
        take(input[*ii], ii, &mut s, &mut n);
    }
    let mut digits = 0;
    while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
        digits += 1;
        take(input[*ii], ii, &mut s, &mut n);
    }
    if *ii < input.len() && n < width && input[*ii] == '.' {
        take('.', ii, &mut s, &mut n);
        while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
            digits += 1;
            take(input[*ii], ii, &mut s, &mut n);
        }
    }
    if digits == 0 {
        *ii = start;
        return None;
    }
    if *ii < input.len() && n < width && matches!(input[*ii], 'e' | 'E') {
        let mut probe = *ii + 1;
        if probe < input.len() && (input[probe] == '+' || input[probe] == '-') {
            probe += 1;
        }
        if probe < input.len() && input[probe].is_ascii_digit() {
            take(input[*ii], ii, &mut s, &mut n); // 'e'
            if *ii < input.len() && (input[*ii] == '+' || input[*ii] == '-') {
                take(input[*ii], ii, &mut s, &mut n);
            }
            while *ii < input.len() && n < width && input[*ii].is_ascii_digit() {
                take(input[*ii], ii, &mut s, &mut n);
            }
        }
    }
    let val: f64 = s.parse().ok()?;
    Some(crate::interp::obj_bytes(crate::obj::new_double_obj(val)))
}

/// `%[...]`: a run of characters in (or, with a leading `^`, not in) the set.
/// `fi` points just past the `[`; it is advanced past the closing `]`.
fn scan_set(
    input: &[char],
    ii: &mut usize,
    width: usize,
    fmt: &[char],
    fi: &mut usize,
) -> Option<Scanned> {
    let mut negate = false;
    if *fi < fmt.len() && fmt[*fi] == '^' {
        negate = true;
        *fi += 1;
    }
    // Collect set members (a `]` immediately after `[`/`[^` is a literal member).
    let mut members: Vec<char> = Vec::new();
    let mut ranges: Vec<(char, char)> = Vec::new();
    let mut first = true;
    while *fi < fmt.len() {
        let c = fmt[*fi];
        if c == ']' && !first {
            *fi += 1;
            break;
        }
        first = false;
        // A range `a-z` (but not a trailing `-`).
        if *fi + 2 < fmt.len() && fmt[*fi + 1] == '-' && fmt[*fi + 2] != ']' {
            ranges.push((c, fmt[*fi + 2]));
            *fi += 3;
        } else {
            members.push(c);
            *fi += 1;
        }
    }
    let in_set = |c: char| {
        members.contains(&c)
            || ranges
                .iter()
                .any(|&(a, b)| (a..=b).contains(&c) || (b..=a).contains(&c))
    };
    let start = *ii;
    let mut n = 0;
    while *ii < input.len() && n < width {
        let c = input[*ii];
        if in_set(c) != negate {
            *ii += 1;
            n += 1;
        } else {
            break;
        }
    }
    if *ii == start {
        return None;
    }
    Some(chars_to_bytes(&input[start..*ii]))
}

fn chars_to_bytes(chars: &[char]) -> Vec<u8> {
    chars.iter().collect::<String>().into_bytes()
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
        assert_eq!(
            counters::finalize(),
            0,
            "residual: {} objs, {} bufs",
            counters::live_objs(),
            counters::live_bufs()
        );
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
    fn scan_basic_conversions() {
        leak_free(|i| {
            assert_eq!(ok(i, b"scan {42 abc} {%d %s} a b"), b"2");
            assert_eq!(ok(i, b"set a"), b"42");
            assert_eq!(ok(i, b"set b"), b"abc");
            assert_eq!(ok(i, b"scan 0xff %x v"), b"1");
            assert_eq!(ok(i, b"set v"), b"255");
            assert_eq!(ok(i, b"scan Z %c c"), b"1");
            assert_eq!(ok(i, b"set c"), b"90");
            // inline mode returns the values as a list.
            assert_eq!(ok(i, b"scan {12 34} {%d %d}"), b"12 34");
            // scanset.
            assert_eq!(ok(i, b"scan hello123 {%[a-z]} w"), b"1");
            assert_eq!(ok(i, b"set w"), b"hello");
            // EOF before any conversion → -1.
            assert_eq!(ok(i, b"scan {} %d x"), b"-1");
            i.eval_str(b"unset -nocomplain a b c v w x");
        });
    }
}
