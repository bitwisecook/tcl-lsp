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

//! `format` — the `sprintf` analogue, per `tmp/tcl9.0.3/generic/tclStringObj.c`
//! (`Tcl_FormatObjCmd` / `Tcl_AppendFormatToObj`).
//!
//! Conversions: `%d`/`%i`/`%u`/`%o`/`%x`/`%X`/`%b`/`%c`/`%s`/`%e`/`%E`/`%f`/
//! `%g`/`%G`/`%%`, with flags (`-`, `+`, space, `0`, `#`), a width and
//! precision (each a literal or `*` taken from an argument), positional
//! specifiers (`%2$d`), and ignored size modifiers (`h`/`l`/`ll`/`L`/…).
//! Integers are scanned as i64 (the bignum widening of `%d` is a follow-up).

use crate::interp::{obj_bytes, Code, Interp};
use crate::obj::TclObj;

/// Register `format`.
pub fn install(interp: &mut Interp) {
    interp.register_builtin(b"format", format_cmd);
}

fn err(interp: &mut Interp, msg: &[u8]) -> Code {
    interp.set_error(msg)
}

/// The largest field width `format` will allocate; beyond it the result would
/// exceed the maximum size of a Tcl value (`INT_MAX`, matching tclsh 9.0 — a
/// width of 2e9 formats, 4.29e9 errors).
const MAX_FORMAT_SIZE: usize = i32::MAX as usize;

/// The integer width a size modifier selects, matching C's `Tcl_AppendFormatToObj`
/// (`tclStringObj.c`): the default is a 32-bit `int`, `h` a 16-bit `short`, `l`
/// (and `j`/`q`/`t`/`z`/`I64`) a 64-bit wide, and `ll`/`L` an arbitrary-precision
/// bignum. The width decides both the signed value of a `%d` and the unsigned bit
/// pattern of a `%u`/`%o`/`%x`/`%X`/`%b` (RUST_ISSUE_091).
#[derive(Clone, Copy, PartialEq, Eq)]
enum IntSize {
    Short,
    Int,
    Wide,
    Big,
}

struct Spec {
    minus: bool,
    plus: bool,
    space: bool,
    zero: bool,
    alt: bool,
    width: usize,
    has_width: bool,
    precision: Option<usize>,
    size: IntSize,
}

fn format_cmd(interp: &mut Interp, argv: &[*mut TclObj]) -> Code {
    if argv.len() < 2 {
        let mut m = b"wrong # args: should be \"".to_vec();
        m.extend_from_slice(b"format formatString ?arg ...?\"");
        return interp.set_error(&m);
    }
    let f = obj_bytes(argv[1]);
    let args = &argv[2..];
    let mut argi = 0usize;
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    // XPG3 `%n$` positional specifiers may not be mixed with plain `%` sequential
    // ones (C's `Tcl_AppendFormatToObj`); these track which style has been seen.
    let mut used_xpg = false;
    let mut used_seq = false;

    // Fetch the next sequential / explicit argument as bytes.
    macro_rules! next_arg {
        ($interp:expr, $args:expr, $argi:expr, $explicit:expr) => {{
            let idx = match $explicit {
                Some(n) => n,
                None => {
                    let n = $argi;
                    $argi += 1;
                    n
                }
            };
            match $args.get(idx) {
                Some(&a) => obj_bytes(a),
                None => return err($interp, b"not enough arguments for all format specifiers"),
            }
        }};
    }

    while i < f.len() {
        if f[i] != b'%' {
            out.push(f[i]);
            i += 1;
            continue;
        }
        i += 1;
        if f.get(i) == Some(&b'%') {
            out.push(b'%');
            i += 1;
            continue;
        }

        // Positional specifier `N$`.
        let mut explicit: Option<usize> = None;
        let save = i;
        let (num, got) = read_uint(&f, &mut i);
        if got && f.get(i) == Some(&b'$') {
            explicit = Some(num.saturating_sub(1));
            i += 1;
        } else {
            i = save;
        }

        // Flags.
        let mut spec = Spec {
            minus: false,
            plus: false,
            space: false,
            zero: false,
            alt: false,
            width: 0,
            has_width: false,
            precision: None,
            size: IntSize::Int,
        };
        loop {
            match f.get(i) {
                Some(b'-') => spec.minus = true,
                Some(b'+') => spec.plus = true,
                Some(b' ') => spec.space = true,
                Some(b'0') => spec.zero = true,
                Some(b'#') => spec.alt = true,
                _ => break,
            }
            i += 1;
        }

        // Width (literal or `*`).
        if f.get(i) == Some(&b'*') {
            i += 1;
            let bytes = next_arg!(interp, args, argi, None);
            match parse_i64(&bytes) {
                Some(w) if w < 0 => {
                    spec.minus = true;
                    spec.width = (-w) as usize;
                }
                Some(w) => spec.width = w as usize,
                None => return err(interp, b"expected integer but got bad width"),
            }
            spec.has_width = true;
        } else {
            let (w, got) = read_uint(&f, &mut i);
            if got {
                spec.width = w;
                spec.has_width = true;
            }
        }
        // A width that would make the result exceed Tcl's value-size limit is an
        // error, not an enormous allocation (which would overflow `Vec`'s
        // capacity and panic). Mirrors C's `format` overflow check
        // (`tclStringObj.c`); verified vs tclsh 9.0 (`format-19.4.x`).
        if spec.has_width && spec.width > MAX_FORMAT_SIZE {
            return err(interp, b"max size for a Tcl value exceeded");
        }

        // Precision.
        if f.get(i) == Some(&b'.') {
            i += 1;
            if f.get(i) == Some(&b'*') {
                i += 1;
                let bytes = next_arg!(interp, args, argi, None);
                spec.precision = Some(parse_i64(&bytes).unwrap_or(0).max(0) as usize);
            } else {
                let (p, _) = read_uint(&f, &mut i);
                spec.precision = Some(p);
            }
        }

        // Size modifier (C's step 5). Exactly one group is consumed, so a stray
        // second modifier char (`%hhx`) falls through to become the conversion
        // char and errors, as tclsh does.
        match f.get(i) {
            Some(b'h') => {
                spec.size = IntSize::Short;
                i += 1;
            }
            Some(b'l') => {
                i += 1;
                if f.get(i) == Some(&b'l') {
                    spec.size = IntSize::Big;
                    i += 1;
                } else {
                    spec.size = IntSize::Wide;
                }
            }
            Some(b'q' | b'j') => {
                spec.size = IntSize::Wide;
                i += 1;
            }
            Some(b't' | b'z') => {
                // 64-bit `size_t`/`ptrdiff_t` on every platform we target.
                spec.size = IntSize::Wide;
                i += 1;
            }
            Some(b'L') => {
                spec.size = IntSize::Big;
                i += 1;
            }
            Some(b'I') => {
                if f.get(i + 1) == Some(&b'6') && f.get(i + 2) == Some(&b'4') {
                    spec.size = IntSize::Wide;
                    i += 3;
                } else if f.get(i + 1) == Some(&b'3') && f.get(i + 2) == Some(&b'2') {
                    spec.size = IntSize::Int;
                    i += 3;
                } else {
                    i += 1;
                }
            }
            _ => {}
        }

        let Some(&conv) = f.get(i) else {
            return err(interp, b"format string ended in middle of field specifier");
        };
        i += 1;

        // Fetch this conversion's argument, enforcing the XPG3 rule: a `%n$`
        // positional index must be in range and must not be mixed with plain
        // sequential `%` specifiers (format-11.x), matching C's messages.
        let arg = match explicit {
            Some(n) => {
                if used_seq {
                    return err(
                        interp,
                        b"cannot mix \"%\" and \"%n$\" conversion specifiers",
                    );
                }
                used_xpg = true;
                match args.get(n) {
                    Some(&a) => obj_bytes(a),
                    None => return err(interp, b"\"%n$\" argument index out of range"),
                }
            }
            None => {
                if used_xpg {
                    return err(
                        interp,
                        b"cannot mix \"%\" and \"%n$\" conversion specifiers",
                    );
                }
                used_seq = true;
                let n = argi;
                argi += 1;
                match args.get(n) {
                    Some(&a) => obj_bytes(a),
                    None => {
                        return err(interp, b"not enough arguments for all format specifiers");
                    }
                }
            }
        };
        match render(interp, conv, &spec, &arg) {
            Ok(field) => out.extend_from_slice(&field),
            Err(code) => return code,
        }
    }

    interp.set_result_bytes(&out);
    Code::Ok
}

/// Render one conversion into its (already width/justify-applied) bytes.
fn render(interp: &mut Interp, conv: u8, spec: &Spec, arg: &[u8]) -> Result<Vec<u8>, Code> {
    match conv {
        b'd' | b'i' | b'u' | b'o' | b'x' | b'X' | b'b' => {
            let v = parse_i64(arg).ok_or_else(|| bad_int(interp, arg))?;
            int_field(interp, spec, v, conv)
        }
        b'c' => {
            let v = parse_i64(arg).ok_or_else(|| bad_int(interp, arg))?;
            let ch = char::from_u32(v as u32).unwrap_or('\u{fffd}');
            Ok(pad(spec, ch.to_string().into_bytes(), false))
        }
        b's' => {
            let mut s = arg.to_vec();
            if let Some(p) = spec.precision {
                // Precision limits to `p` characters.
                let chars: Vec<char> = String::from_utf8_lossy(&s).chars().take(p).collect();
                s = chars.into_iter().collect::<String>().into_bytes();
            }
            Ok(pad(spec, s, false))
        }
        b'e' | b'E' | b'f' | b'g' | b'G' => {
            let v = parse_f64(arg).ok_or_else(|| {
                let mut m = b"expected floating-point number but got ".to_vec();
                m.extend_from_slice(bad_value_desc(arg).as_bytes());
                interp.set_error(&m)
            })?;
            Ok(float_field(spec, conv, v))
        }
        _ => {
            let mut m = b"bad field specifier \"".to_vec();
            m.push(conv);
            m.push(b'"');
            Err(interp.set_error(&m))
        }
    }
}

fn bad_int(interp: &mut Interp, arg: &[u8]) -> Code {
    let mut m = b"expected integer but got ".to_vec();
    m.extend_from_slice(bad_value_desc(arg).as_bytes());
    interp.set_error(&m)
}

/// The `got <here>` tail of a numeric-coercion error: a multi-token, well-formed
/// list reads as `a list`, any other value is quoted (truncated to 50 bytes) —
/// matching C (`tclStrToD.c` formaterr). Shared with the VM via
/// [`tcl_syntax::list::describe_bad_value`].
fn bad_value_desc(arg: &[u8]) -> String {
    tcl_syntax::list::describe_bad_value(&String::from_utf8_lossy(arg))
}

/// Assemble an integer field for `%d`/`%i`/`%u`/`%o`/`%x`/`%X`/`%b`.
///
/// The size modifier (`spec.size`) chooses the width the value is reduced to
/// before conversion, exactly as C's `Tcl_AppendFormatToObj` does: `%d`/`%i`
/// render the *signed* value at that width, while `%u`/`%o`/`%x`/`%X`/`%b`
/// render its *unsigned* bit pattern — so `%x -1` is `ffffffff` at the default
/// 32-bit width and `ffffffffffffffff` with `l`. A sign (`-`/`+`/space) is
/// carried only by `%d`/`%i` and by any bignum conversion; a negative bignum
/// under `%u` is the `unsigned bignum format is invalid` error (RUST_ISSUE_091).
fn int_field(interp: &mut Interp, spec: &Spec, value: i64, conv: u8) -> Result<Vec<u8>, Code> {
    let (radix, upper) = match conv {
        b'o' => (8u32, false),
        b'x' => (16, false),
        b'X' => (16, true),
        b'b' => (2, false),
        _ => (10, false), // d / i / u
    };
    // The signed value and the unsigned bit pattern at the selected width.
    let (signed_val, unsigned_bits): (i128, u128) = match spec.size {
        IntSize::Short => (i128::from(value as i16), u128::from(value as i16 as u16)),
        IntSize::Int => (i128::from(value as i32), u128::from(value as i32 as u32)),
        IntSize::Wide => (i128::from(value), u128::from(value as u64)),
        IntSize::Big => (i128::from(value), i128::from(value).unsigned_abs()),
    };
    let signed_conv = matches!(conv, b'd' | b'i');
    let (negative, magnitude): (bool, u128) = if signed_conv {
        (signed_val < 0, signed_val.unsigned_abs())
    } else if spec.size == IntSize::Big {
        // A negative bignum has no unsigned form; `%o`/`%x`/`%X`/`%b` show its
        // sign and magnitude, but `%u` is an error.
        if signed_val < 0 && conv == b'u' {
            return Err(interp.set_error(b"unsigned bignum format is invalid"));
        }
        (signed_val < 0, signed_val.unsigned_abs())
    } else {
        (false, unsigned_bits)
    };

    let mut digits = to_radix(magnitude, radix, upper);
    if let Some(p) = spec.precision {
        while digits.len() < p {
            digits.insert(0, b'0');
        }
    }
    // Only `%d`/`%i` and bignum conversions carry a sign; an unsigned fixed-width
    // conversion never emits `-`/`+`/space (matching C's `useBig || ch=='d'`).
    let carries_sign = signed_conv || spec.size == IntSize::Big;
    let sign: &[u8] = if carries_sign {
        if negative {
            b"-"
        } else if spec.plus {
            b"+"
        } else if spec.space {
            b" "
        } else {
            b""
        }
    } else {
        b""
    };
    // `#` alternate-form prefix (Tcl 9: `0o`/`0x`/`0X`/`0b`), suppressed when the
    // value is zero and never applied to decimal conversions.
    let prefix: &[u8] = if spec.alt && magnitude != 0 {
        match conv {
            // Tcl 9 uses a lowercase `0x` prefix even for `%#X` (only the digits
            // uppercase): `format %#X 12` → `0xC` (`tclStringObj.c`).
            b'o' => b"0o",
            b'x' | b'X' => b"0x",
            b'b' => b"0b",
            _ => b"",
        }
    } else {
        b""
    };

    let mut body = Vec::new();
    body.extend_from_slice(sign);
    body.extend_from_slice(prefix);
    body.extend_from_slice(&digits);

    // Zero-padding (only when right-justified and no explicit precision).
    if spec.zero
        && !spec.minus
        && spec.precision.is_none()
        && spec.has_width
        && spec.width > body.len()
    {
        let pad_n = spec.width - body.len();
        let mut padded = Vec::new();
        padded.extend_from_slice(sign);
        padded.extend_from_slice(prefix);
        padded.extend(std::iter::repeat_n(b'0', pad_n));
        padded.extend_from_slice(&digits);
        return Ok(padded);
    }
    Ok(pad(spec, body, true))
}

/// Float field via Rust formatting, then width/justification.
fn float_field(spec: &Spec, conv: u8, value: f64) -> Vec<u8> {
    let prec = spec.precision.unwrap_or(6);
    let mut s = match conv {
        b'f' => format!("{value:.prec$}"),
        b'e' => format!("{value:.prec$e}"),
        b'E' => format!("{value:.prec$E}"),
        b'g' | b'G' => format_g(value, prec.max(1), conv == b'G', spec.alt),
        _ => format!("{value}"),
    };
    // Normalise Rust's exponent (`1e2` → `1e+02`) toward C's form.
    if matches!(conv, b'e' | b'E') {
        s = fix_exponent(&s);
    }
    let negative = s.starts_with('-');
    let mut bytes = s.into_bytes();
    if !negative {
        if spec.plus {
            bytes.insert(0, b'+');
        } else if spec.space {
            bytes.insert(0, b' ');
        }
    }
    if spec.zero && !spec.minus && spec.has_width && spec.width > bytes.len() {
        let sign_len = usize::from(matches!(bytes.first(), Some(b'-' | b'+' | b' ')));
        let pad_n = spec.width - bytes.len();
        let mut padded = bytes[..sign_len].to_vec();
        padded.extend(std::iter::repeat_n(b'0', pad_n));
        padded.extend_from_slice(&bytes[sign_len..]);
        return padded;
    }
    pad(spec, bytes, true)
}

/// Trailing-zero trim for `%g`.
fn trim_g(s: &str) -> String {
    if s.contains('.') {
        let t = s.trim_end_matches('0');
        t.trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// `%g`/`%G` with `p` **significant** digits (C's `printf`): use `%e` when the
/// decimal exponent is `< -4` or `>= p`, else `%f`, each at the precision that
/// yields `p` significant digits; trailing zeros are trimmed unless `alt` (`#`).
fn format_g(value: f64, p: usize, upper: bool, alt: bool) -> String {
    if !value.is_finite() {
        let s = format!("{value}"); // inf / NaN
        return if upper { s.to_uppercase() } else { s };
    }
    // Significant-digit scientific form gives the decimal exponent directly.
    let sci = format!("{value:.*e}", p - 1);
    let exp: i32 = sci
        .find('e')
        .and_then(|i| sci[i + 1..].parse().ok())
        .unwrap_or(0);
    let mut out = if exp < -4 || exp >= p as i32 {
        // %e with p-1 fractional digits; trim the mantissa (keep the exponent).
        let s = fix_exponent(&sci);
        if alt {
            s
        } else {
            trim_g_e(&s)
        }
    } else {
        // %f with enough fractional digits for p significant figures.
        let fprec = usize::try_from(p as i32 - 1 - exp).unwrap_or(0);
        let s = format!("{value:.fprec$}");
        if alt {
            s
        } else {
            trim_g(&s)
        }
    };
    if upper {
        out = out.to_uppercase();
    }
    out
}

/// Trailing-zero trim of a `%e`-form `%g` result: trim zeros in the mantissa
/// (before `e`), keeping the exponent intact.
fn trim_g_e(s: &str) -> String {
    let Some(epos) = s.find(['e', 'E']) else {
        return trim_g(s);
    };
    let (mantissa, exp) = s.split_at(epos);
    format!("{}{}", trim_g(mantissa), exp)
}

/// Rust prints `1e2`; C prints `1.000000e+02`-style with a signed 2-digit
/// exponent. Normalise the sign + min-2-digit padding.
fn fix_exponent(s: &str) -> String {
    let Some(pos) = s.find(['e', 'E']) else {
        return s.to_string();
    };
    let (mantissa, exp) = s.split_at(pos);
    let e = &exp[..1];
    let rest = &exp[1..];
    let (sign, digits) = match rest.strip_prefix('-') {
        Some(d) => ("-", d),
        None => ("+", rest.strip_prefix('+').unwrap_or(rest)),
    };
    let digits = if digits.len() < 2 {
        format!("{digits:0>2}")
    } else {
        digits.to_string()
    };
    format!("{mantissa}{e}{sign}{digits}")
}

/// Apply width + justification to a fully-rendered body. `numeric` controls
/// nothing here (zero-pad is handled by the callers); spaces always pad.
fn pad(spec: &Spec, body: Vec<u8>, numeric: bool) -> Vec<u8> {
    if !spec.has_width || body.len() >= spec.width {
        return body;
    }
    let pad_n = spec.width - body.len();
    let mut out = Vec::with_capacity(spec.width);
    if spec.minus {
        out.extend_from_slice(&body);
        out.extend(std::iter::repeat_n(b' ', pad_n));
    } else {
        // A string/char field with the `0` flag zero-pads on the left, like C's
        // `format %05s a` → `0000a`. Numeric fields do their own sign/precision-
        // aware zero-padding before reaching here, so they always space-pad.
        let fill = if spec.zero && !numeric { b'0' } else { b' ' };
        out.extend(std::iter::repeat_n(fill, pad_n));
        out.extend_from_slice(&body);
    }
    out
}

fn to_radix(mut v: u128, radix: u32, upper: bool) -> Vec<u8> {
    if v == 0 {
        return vec![b'0'];
    }
    let digits = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut out = Vec::new();
    while v > 0 {
        out.push(digits[(v % radix as u128) as usize]);
        v /= radix as u128;
    }
    out.reverse();
    out
}

/// Read a run of ASCII digits as a `usize`; reports whether any were read.
fn read_uint(f: &[u8], i: &mut usize) -> (usize, bool) {
    let mut n: usize = 0;
    let mut got = false;
    while let Some(&c) = f.get(*i) {
        if c.is_ascii_digit() {
            got = true;
            // Saturate rather than wrap: an over-large width must still compare
            // greater than the value-size limit (see `MAX_FORMAT_SIZE`), not
            // wrap around to a small number.
            n = n.saturating_mul(10).saturating_add((c - b'0') as usize);
            *i += 1;
        } else {
            break;
        }
    }
    (n, got)
}

/// Parse a Tcl integer via the shared canonical parser (`tcl_syntax::number`),
/// tolerating surrounding whitespace. Going through `parse_whole` rather than
/// rolling our own keeps this faithful to `tclsh`: a doubled sign (`++1`,
/// `--1`) is rejected, unlike a naive strip-one-sign + `i64::from_str_radix`
/// (which would silently re-accept the inner sign). An out-of-`i64`-range
/// magnitude (a `Big`) yields `None` here — the bignum widening of `%d` is a
/// separate follow-up.
fn parse_i64(b: &[u8]) -> Option<i64> {
    let s = core::str::from_utf8(b).ok()?;
    match tcl_syntax::number::parse_whole(s.trim()) {
        Some(tcl_syntax::number::Number::Int(n)) => Some(n),
        _ => None,
    }
}

fn parse_f64(b: &[u8]) -> Option<f64> {
    core::str::from_utf8(b).ok()?.trim().parse::<f64>().ok()
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
    fn format_conversions_flags_width() {
        leak_free(|i| {
            assert_eq!(
                ok(i, b"format {%d %05d %-5d|} 42 42 42"),
                b"42 00042 42   |"
            );
            assert_eq!(ok(i, b"format {%x %#x %X} 255 255 255"), b"ff 0xff FF");
            assert_eq!(ok(i, b"format {%5.2f} 3.14159"), b" 3.14");
            assert_eq!(ok(i, b"format {%.3s %10s|} hello hi"), b"hel         hi|");
            assert_eq!(ok(i, b"format {%c%c%c} 72 105 33"), b"Hi!");
            // positional specifiers.
            assert_eq!(ok(i, b"format {%2$s %1$s} a b"), b"b a");
            assert_eq!(ok(i, b"format {%+d % d %o} 5 5 8"), b"+5  5 10");
        });
    }

    #[test]
    fn format_overflow_width_errors_not_panics() {
        // A width past the value-size limit is a clean error, not a `Vec`
        // capacity-overflow panic / a multi-GB allocation (bug d498578df4,
        // `format-19.4.x`). Verified vs tclsh 9.0.
        leak_free(|i| {
            for w in [b"4294967294".as_slice(), b"18446744073709551614"] {
                let mut src = b"format %".to_vec();
                src.extend_from_slice(w);
                src.extend_from_slice(b"g 0");
                assert_eq!(i.eval_str(&src), Code::Error);
                assert_eq!(i.result_bytes(), b"max size for a Tcl value exceeded");
            }
            // A merely-large-but-valid width still formats.
            assert_eq!(ok(i, b"format %5s hi"), b"   hi");
        });
    }
}
