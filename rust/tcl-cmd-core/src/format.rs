//! Portable `format` command logic, generic over [`ValueOps`].
//!
//! The conversion-specifier grammar is already shared
//! ([`tcl_syntax::format::parse_spec`]); this is the rendering half — applying a
//! parsed [`Spec`] to a value via [`ValueOps`] coercion. The numeric/padding
//! helpers are pure (no value model), so only [`render_spec`] takes `ops`.
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps

use tcl_syntax::format::{FmtFlags, Spec, parse_spec};
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// `format formatString ?arg ...?`.
pub fn format_cmd<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    let Some((fmt, rest)) = args.split_first() else {
        return Err(CmdError::wrong_args("format formatString ?arg ...?"));
    };
    let fmt = ops.as_str(fmt).to_string();
    let rendered = render(ops, &fmt, rest)?;
    Ok(ops.new_string(rendered))
}

/// Render `fmt` against `args`, consuming arguments left-to-right.
fn render<O: ValueOps>(ops: &mut O, fmt: &str, args: &[O::Value]) -> Result<String, CmdError> {
    let bytes = fmt.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut ai = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            // Copy a whole UTF-8 char, not a single byte.
            let ch_len = utf8_len(bytes[i]);
            out.push_str(&fmt[i..(i + ch_len).min(bytes.len())]);
            i += ch_len;
            continue;
        }
        if bytes.get(i + 1) == Some(&b'%') {
            out.push('%');
            i += 2;
            continue;
        }
        let mut j = i + 1;
        let Some(spec) = parse_spec(bytes, &mut j) else {
            // Unmodelled spec (e.g. `*` width) — emit the `%` literally.
            out.push('%');
            i += 1;
            continue;
        };
        if spec.verb != b'%' {
            let Some(arg) = args.get(ai) else {
                return Err(CmdError::new(
                    "not enough arguments for all format specifiers",
                ));
            };
            out.push_str(&render_spec(ops, &spec, arg)?);
            ai += 1;
        }
        i = j;
    }
    Ok(out)
}

fn utf8_len(b: u8) -> usize {
    match b {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

/// Render one conversion against `arg`.
fn render_spec<O: ValueOps>(ops: &mut O, spec: &Spec, arg: &O::Value) -> Result<String, CmdError> {
    let verb = spec.verb;
    match verb {
        b'd' | b'i' | b'u' => {
            let n = ops.as_int(arg)?;
            Ok(pad_number(
                &int_digits(n, spec),
                n < 0 && verb != b'u',
                spec,
            ))
        }
        b'x' | b'X' | b'o' | b'b' => {
            let n = ops.as_int(arg)?;
            Ok(pad_number(&based_digits(n, spec), false, spec))
        }
        b'c' => {
            let n = ops.as_int(arg)?;
            let ch = u32::try_from(n)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(String::new, |c| c.to_string());
            Ok(justify(&ch, spec))
        }
        b'f' | b'e' | b'E' | b'g' | b'G' => {
            let x = ops.as_double(arg)?;
            Ok(pad_number(
                &float_digits(x, spec),
                x.is_sign_negative(),
                spec,
            ))
        }
        b's' => {
            let mut s = ops.as_str(arg).to_string();
            if let Some(p) = spec.precision {
                s = s.chars().take(p).collect();
            }
            Ok(justify(&s, spec))
        }
        other => Err(CmdError::new(format!(
            "bad field specifier \"{}\"",
            char::from(other)
        ))),
    }
}

/// Decimal digits (no sign) for an integer, honouring `.precision`.
fn int_digits(n: i64, spec: &Spec) -> String {
    apply_precision(n.unsigned_abs().to_string(), spec)
}

/// Digits for `x`/`X`/`o`/`b`, with the `#` alternate-form prefix.
fn based_digits(n: i64, spec: &Spec) -> String {
    #[allow(clippy::cast_sign_loss)]
    let u = n as u64;
    let (mut body, prefix) = match spec.verb {
        b'x' => (
            format!("{u:x}"),
            if spec.flags.contains(FmtFlags::HASH) && u != 0 {
                "0x"
            } else {
                ""
            },
        ),
        b'X' => (
            format!("{u:X}"),
            if spec.flags.contains(FmtFlags::HASH) && u != 0 {
                "0X"
            } else {
                ""
            },
        ),
        b'o' => (
            format!("{u:o}"),
            if spec.flags.contains(FmtFlags::HASH) {
                "0"
            } else {
                ""
            },
        ),
        _ => (format!("{u:b}"), ""),
    };
    body = apply_precision(body, spec);
    format!("{prefix}{body}")
}

/// Magnitude digits for a float verb (sign handled by `pad_number`).
fn float_digits(x: f64, spec: &Spec) -> String {
    let prec = spec.precision.unwrap_or(6);
    let m = x.abs();
    match spec.verb {
        b'f' => format!("{m:.prec$}"),
        b'e' => format!("{m:.prec$e}"),
        b'E' => format!("{m:.prec$E}"),
        // g/G: approximate with bounded precision, trimming trailing zeros.
        _ => {
            let s = format!("{m:.prec$}");
            if s.contains('.') {
                s.trim_end_matches('0').trim_end_matches('.').to_string()
            } else {
                s
            }
        }
    }
}

/// Left-pad `mag` with `0` up to `.precision` digits.
fn apply_precision(mag: String, spec: &Spec) -> String {
    match spec.precision {
        Some(p) if mag.len() < p => format!("{}{mag}", "0".repeat(p - mag.len())),
        Some(0) if mag == "0" => String::new(),
        _ => mag,
    }
}

/// Apply the sign prefix (`-`/`+`/space) and width to a numeric body.
fn pad_number(body: &str, negative: bool, spec: &Spec) -> String {
    let sign = if negative {
        "-"
    } else if spec.flags.contains(FmtFlags::PLUS) {
        "+"
    } else if spec.flags.contains(FmtFlags::SPACE) {
        " "
    } else {
        ""
    };
    let Some(width) = spec.width else {
        return format!("{sign}{body}");
    };
    let len = sign.len() + body.len();
    if len >= width {
        return format!("{sign}{body}");
    }
    let pad = width - len;
    if spec.flags.contains(FmtFlags::MINUS) {
        format!("{sign}{body}{}", " ".repeat(pad))
    } else if spec.flags.contains(FmtFlags::ZERO) && spec.precision.is_none() {
        format!("{sign}{}{body}", "0".repeat(pad))
    } else {
        format!("{}{sign}{body}", " ".repeat(pad))
    }
}

/// Apply width / left-justify to a string conversion.
fn justify(s: &str, spec: &Spec) -> String {
    let Some(width) = spec.width else {
        return s.to_string();
    };
    let len = s.chars().count();
    if len >= width {
        return s.to_string();
    }
    let pad = " ".repeat(width - len);
    if spec.flags.contains(FmtFlags::MINUS) {
        format!("{s}{pad}")
    } else {
        format!("{pad}{s}")
    }
}
