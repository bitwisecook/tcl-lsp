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

//! Portable `format` command logic, generic over [`ValueOps`].
//!
//! The conversion-specifier grammar is already shared
//! ([`tcl_syntax::format::parse_spec`]); this is the rendering half — applying a
//! parsed [`Spec`] to a value via [`ValueOps`] coercion. The numeric/padding
//! helpers are pure (no value model), so only [`render_spec`] takes `ops`.
//!
//! [`ValueOps`]: tcl_syntax::value::ValueOps

use tcl_syntax::format::{FmtFlags, Spec, parse_spec_with_limit};
use tcl_syntax::value::ValueOps;

use crate::error::CmdError;

/// Whether `verb` belongs to the format conversion grammar consumed by the
/// shared renderer.  This forwards the parser owner rather than maintaining a
/// second editor-only verb table.
#[must_use]
pub fn is_verb(verb: u8) -> bool {
    tcl_syntax::format::is_verb(verb)
}

/// Whether a parsed conversion is available under one resolved profile.
/// Version/dialect policy remains in the profile-aware syntax owner.
#[must_use]
pub fn is_available(spec: &Spec, profile: &tcl_dialect::DialectProfile) -> bool {
    tcl_syntax::format::is_available(spec, profile)
}

/// `format formatString ?arg ...?`.
pub fn format_cmd<O: ValueOps>(ops: &mut O, args: &[O::Value]) -> Result<O::Value, CmdError> {
    format_cmd_with_syntax(ops, args, tcl_syntax::number::runtime_syntax())
}

/// `format` under an explicitly resolved numeral/alternate-prefix grammar.
/// Runtime adapters use this entry point so an interpreter pinned to Tcl 8.x
/// does not inherit Tcl 9's `0d`/`0o` alternate forms.
pub fn format_cmd_with_syntax<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
    syntax: tcl_dialect::NumberSyntax,
) -> Result<O::Value, CmdError> {
    let Some((fmt, rest)) = args.split_first() else {
        return Err(CmdError::wrong_args("format formatString ?arg ...?"));
    };
    let fmt = ops.as_str(fmt).to_string();
    let rendered = render(ops, &fmt, rest, syntax)?;
    Ok(ops.new_string(rendered))
}

const MAX_RUNTIME_FIELD: usize = i32::MAX as usize;

/// Render `fmt` against `args`, consuming arguments left-to-right.
fn render<O: ValueOps>(
    ops: &mut O,
    fmt: &str,
    args: &[O::Value],
    syntax: tcl_dialect::NumberSyntax,
) -> Result<String, CmdError> {
    let bytes = fmt.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    let mut ai = 0;
    // Tcl forbids mixing positional (`%n$`) and sequential conversions in one
    // format string ("cannot mix …"); track which mode the format has committed
    // to so the second kind errors.
    let mut saw_positional = false;
    let mut saw_sequential = false;
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
        let Some(mut spec) = parse_spec_with_limit(bytes, &mut j, usize::MAX) else {
            // A `%` that is not `%%` always begins a conversion — Tcl has no
            // literal lone `%`. An incomplete one (`%`, `%5`, a trailing `%`)
            // has no verb to consume an argument, which Tcl reports as
            // "not enough arguments".
            return Err(CmdError::new(
                "not enough arguments for all format specifiers",
            ));
        };
        if spec.width.is_some_and(|width| width > MAX_RUNTIME_FIELD)
            || spec
                .precision
                .is_some_and(|precision| precision > MAX_RUNTIME_FIELD)
        {
            return Err(CmdError::new("max size for a Tcl value exceeded"));
        }
        if spec.verb != b'%' {
            // Commit the format to positional or sequential mode and reject a mix
            // (`format {%2$d %d} …` is a Tcl error, not arg-2-then-arg-1).
            if spec.arg_index.is_some() {
                if saw_sequential {
                    return Err(CmdError::new(
                        "cannot mix \"%\" and \"%n$\" conversion specifiers",
                    ));
                }
                saw_positional = true;
            } else {
                if saw_positional {
                    return Err(CmdError::new(
                        "cannot mix \"%\" and \"%n$\" conversion specifiers",
                    ));
                }
                saw_sequential = true;
            }
            // A positional `%n$` spec consumes consecutively starting at `n-1`
            // (the `*` width, then the `.*` precision, then the value), leaving
            // the sequential cursor untouched; an ordinary spec consumes from the
            // running cursor `ai`. So `%2$*d` takes its width from arg 2 and its
            // value from arg 3 (format-... ), matching tclsh.
            let positional = spec.arg_index.is_some();
            let mut cur = match spec.arg_index {
                Some(n) => n
                    .checked_sub(1)
                    .ok_or_else(|| CmdError::new("\"%n$\" argument index out of range"))?,
                None => ai,
            };
            // `*` width / `.*` precision take their value from a preceding
            // argument (format-1.1/1.6). A negative `*` width left-justifies.
            if spec.width_star {
                let w = ops.as_int(take_arg(args, &mut cur, positional)?)?;
                if w < 0 {
                    spec.flags |= FmtFlags::MINUS;
                }
                let mag = usize::try_from(w.unsigned_abs()).unwrap_or(usize::MAX);
                if mag > MAX_RUNTIME_FIELD {
                    return Err(CmdError::new("max size for a Tcl value exceeded"));
                }
                spec.width = Some(mag);
            }
            if spec.precision_star {
                let p = ops.as_int(take_arg(args, &mut cur, positional)?)?;
                // A negative `.*` precision is treated as omitted (C).
                spec.precision = usize::try_from(p).ok().map(|p| p.min(MAX_RUNTIME_FIELD));
            }
            let arg = take_arg(args, &mut cur, positional)?;
            out.push_str(&render_spec(ops, &spec, arg, syntax)?);
            if !positional {
                ai = cur;
            }
        }
        i = j;
    }
    Ok(out)
}

/// Take the argument at `*cur` and advance the cursor. Shared by the `*`/`.*`
/// width and the value consumption, in both the sequential and the positional
/// (`%n$`) paths — the two differ only in the out-of-range message tclsh uses.
fn take_arg<'a, V>(args: &'a [V], cur: &mut usize, positional: bool) -> Result<&'a V, CmdError> {
    let arg = args.get(*cur).ok_or_else(|| {
        CmdError::new(if positional {
            "\"%n$\" argument index out of range"
        } else {
            "not enough arguments for all format specifiers"
        })
    })?;
    *cur += 1;
    Ok(arg)
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
fn render_spec<O: ValueOps>(
    ops: &mut O,
    spec: &Spec,
    arg: &O::Value,
    syntax: tcl_dialect::NumberSyntax,
) -> Result<String, CmdError> {
    let verb = spec.verb;
    match verb {
        b'd' | b'i' | b'u' => {
            let n = ops.as_int(arg)?;
            let mut digits = int_digits(n, spec);
            // Tcl 9 `%#d` / `%#i` alternate form: a `0d` radix prefix on a
            // non-zero value (dropped for zero, like `%#x 0` → `0`). `%u` takes
            // no prefix. The sign and width are applied around it by
            // `pad_number`, so `%#d -42` → `-0d42`.
            if syntax.has_decimal_prefix()
                && spec.flags.contains(FmtFlags::HASH)
                && matches!(verb, b'd' | b'i')
                && n != 0
            {
                digits.insert_str(0, "0d");
            }
            Ok(pad_number(&digits, n < 0 && verb != b'u', spec))
        }
        b'x' | b'X' | b'o' | b'b' => {
            let n = ops.as_int(arg)?;
            Ok(pad_number(&based_digits(n, spec, syntax), false, spec))
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
fn based_digits(n: i64, spec: &Spec, syntax: tcl_dialect::NumberSyntax) -> String {
    // reinterpret the two's-complement bit pattern as u64: `%x`/`%o`/`%b` of a
    // negative int prints the unsigned representation, matching C's `format`.
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
            // Tcl 8.x follows the verb's case (`0XC`); Tcl 9's explicit
            // radix spelling is lowercase even for `%#X` (`0xC`).
            if spec.flags.contains(FmtFlags::HASH) && u != 0 {
                if syntax.has_decimal_prefix() {
                    "0x"
                } else {
                    "0X"
                }
            } else {
                ""
            },
        ),
        b'o' => (
            format!("{u:o}"),
            // Tcl 9 `%#o` → `0o` radix prefix (8.6 used a bare leading `0`),
            // dropped for zero (`%#o 0` → `0`).
            if spec.flags.contains(FmtFlags::HASH) && u != 0 {
                if syntax.has_decimal_prefix() {
                    "0o"
                } else {
                    "0"
                }
            } else {
                ""
            },
        ),
        _ => (
            format!("{u:b}"),
            // `%#b` → `0b` prefix (Tcl 8.6 and 9.0 both emit it; format-1.x).
            if spec.flags.contains(FmtFlags::HASH) && u != 0 {
                "0b"
            } else {
                ""
            },
        ),
    };
    body = apply_precision(body, spec);
    format!("{prefix}{body}")
}

/// Whether `verb` is a floating-point conversion (`e`/`E`/`f`/`F`/`g`/`G`).
fn is_float_verb(verb: u8) -> bool {
    matches!(verb, b'e' | b'E' | b'f' | b'F' | b'g' | b'G')
}

/// Re-render a Rust exponent (`1.5e4` / `1.5e-4`) in C/Tcl style with an
/// explicit sign and at least two exponent digits (`1.5e+04` / `1.5e-04`).
fn c_style_exp(rust_e: &str) -> String {
    let Some(pos) = rust_e.find(['e', 'E']) else {
        return rust_e.to_string();
    };
    let e_char = &rust_e[pos..=pos];
    let mantissa = &rust_e[..pos];
    let exp: i32 = rust_e[pos + 1..].parse().unwrap_or(0);
    let sign = if exp < 0 { '-' } else { '+' };
    format!("{mantissa}{e_char}{sign}{:02}", exp.abs())
}

/// Magnitude digits for a float verb (sign handled by `pad_number`).
fn float_digits(x: f64, spec: &Spec) -> String {
    let prec = spec.precision.unwrap_or(6);
    let m = x.abs();
    match spec.verb {
        b'f' | b'F' => format!("{m:.prec$}"),
        b'e' => c_style_exp(&format!("{m:.prec$e}")),
        b'E' => c_style_exp(&format!("{m:.prec$E}")),
        // g/G (C semantics): precision P (0 → 1) is the number of significant
        // digits. Using the decimal exponent X (from an %e render at P-1
        // fractional digits), pick %e when X < -4 or X >= P, else %f with
        // P-1-X fractional digits; then strip trailing zeros / a bare `.`
        // (unless the `#` alternate form keeps them).
        _ => {
            let p = prec.max(1);
            let upper = spec.verb == b'G';
            let probe = format!("{m:.*e}", p - 1);
            let exp: i32 = probe
                .find('e')
                .and_then(|i| probe[i + 1..].parse().ok())
                .unwrap_or(0);
            let keep_zeros = spec.flags.contains(FmtFlags::HASH);
            if exp < -4 || exp >= i32::try_from(p).unwrap_or(i32::MAX) {
                let body = c_style_exp(&format!("{m:.*e}", p - 1));
                let out = if keep_zeros { body } else { trim_g_exp(&body) };
                if upper { out.replace('e', "E") } else { out }
            } else {
                let fprec = usize::try_from(i32::try_from(p).unwrap_or(0) - 1 - exp).unwrap_or(0);
                let body = format!("{m:.fprec$}");
                if keep_zeros || !body.contains('.') {
                    body
                } else {
                    body.trim_end_matches('0').trim_end_matches('.').to_string()
                }
            }
        }
    }
}

/// Trim trailing mantissa zeros (and a bare `.`) from a C-style `%e` body
/// without disturbing the exponent: `1.20000e+06` → `1.2e+06`,
/// `1.00000e+06` → `1e+06`.
fn trim_g_exp(body: &str) -> String {
    let Some(epos) = body.find(['e', 'E']) else {
        return body.to_string();
    };
    let (mantissa, exp) = body.split_at(epos);
    let trimmed = if mantissa.contains('.') {
        mantissa.trim_end_matches('0').trim_end_matches('.')
    } else {
        mantissa
    };
    format!("{trimmed}{exp}")
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
    } else if spec.flags.contains(FmtFlags::ZERO)
        && (spec.precision.is_none() || is_float_verb(spec.verb))
    {
        // C ignores the `0` flag with an explicit precision for *integer*
        // conversions, but a float's precision is its fraction width, so `0`
        // still pads (`%08.2f 3.14` → `00003.14`).
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
