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

use tcl_syntax::format::{
    FmtFlags, ParseSpecError, SizeModifier, Spec, parse_spec_with_limit_diagnostic,
};
use tcl_syntax::number::Radix;
use tcl_syntax::value::{IntegerMagnitude, ValueOps};

use crate::error::CmdError;
use crate::value_size;

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
    let fmt = ops.try_as_str(fmt)?.to_string();
    let rendered = render(ops, &fmt, rest, syntax)?;
    Ok(ops.new_string(rendered))
}

const MAX_RUNTIME_FIELD: usize = i32::MAX as usize;

/// C's `overflow` failure (`TCL FORMAT OVERFLOW`): a field the result could
/// not hold.
fn value_too_large() -> CmdError {
    CmdError::with_error_code("max size for a Tcl value exceeded", "TCL FORMAT OVERFLOW")
}

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
    let mut arg_state = FormatArgState::default();
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
        let mut spec = match parse_spec_with_limit_diagnostic(bytes, &mut j, usize::MAX) {
            Ok(Some(spec)) => spec,
            Ok(None) => {
                return Err(CmdError::new(if arg_state.next < args.len() {
                    "format string ended in middle of field specifier"
                } else {
                    "not enough arguments for all format specifiers"
                }));
            }
            Err(ParseSpecError::MissingVerb(partial)) => {
                // A release that predates the modifier reads its first byte as
                // the verb and rejects it before it can report the missing
                // verb (`format %z 1` on 8.6 is `bad field specifier "z"`);
                // one that knows it reports the conversion as incomplete.
                // The parser retains this source fact, so the renderer does
                // not need to re-scan the raw format.
                let mut fields = ConversionFields::from(partial);
                let _ = consume_conversion_args(ops, &mut fields, args, &mut arg_state)?;
                if let Some(size) = partial.size {
                    reject_unknown_modifier(size, syntax)?;
                }
                return Err(CmdError::new(
                    "format string ended in middle of field specifier",
                ));
            }
        };
        // A `%s` precision only truncates, so C takes any that parses.
        if spec.width.is_some_and(|width| width > MAX_RUNTIME_FIELD)
            || (spec.verb != b's'
                && spec
                    .precision
                    .is_some_and(|precision| precision > MAX_RUNTIME_FIELD))
        {
            return Err(value_too_large());
        }
        // A positional `%n$` spec consumes consecutively starting at `n-1`
        // (the `*` width, then the `.*` precision, then the value), leaving
        // the sequential cursor untouched; an ordinary spec consumes from the
        // running cursor `ai`. So `%2$*d` takes its width from arg 2 and its
        // value from arg 3, matching tclsh. The literal `%%` shortcut above
        // is the only percent form that bypasses argument consumption;
        // modified `%…%` forms must reach render_spec and report bad `%`.
        let mut fields = ConversionFields::from(&spec);
        let arg = consume_conversion_args(ops, &mut fields, args, &mut arg_state)?;
        fields.apply_to(&mut spec);
        // A field is at least its width and its argument, and a numeric
        // conversion at least its precision (`%.9d` pads with zeros; a `%s`
        // precision only truncates): all charged before the field is rendered.
        value_size::charge(ops, |ops, cap| {
            let precision = if spec.verb == b's' {
                0
            } else {
                spec.precision.unwrap_or(0)
            };
            let field = ops
                .measure_value(arg, cap)
                .max(spec.width.unwrap_or(0) as u64)
                .max(precision as u64);
            (out.len() as u64).saturating_add(field)
        })?;
        out.push_str(&render_spec(ops, &spec, arg, syntax)?);
        i = j;
    }
    Ok(out)
}

/// The argument-consuming fields shared by complete and incomplete specs.
/// Keeping these in one value avoids a second, subtly different implementation
/// of `*`/`.*` and positional argument handling for parser error paths.
#[derive(Debug, Clone, Copy)]
struct ConversionFields {
    flags: FmtFlags,
    width: Option<usize>,
    precision: Option<usize>,
    width_star: bool,
    precision_star: bool,
    arg_index: Option<usize>,
}

impl From<&Spec> for ConversionFields {
    fn from(spec: &Spec) -> Self {
        Self {
            flags: spec.flags,
            width: spec.width,
            precision: spec.precision,
            width_star: spec.width_star,
            precision_star: spec.precision_star,
            arg_index: spec.arg_index,
        }
    }
}

impl From<tcl_syntax::format::PartialSpec> for ConversionFields {
    fn from(spec: tcl_syntax::format::PartialSpec) -> Self {
        Self {
            flags: spec.flags,
            width: spec.width,
            precision: spec.precision,
            width_star: spec.width_star,
            precision_star: spec.precision_star,
            arg_index: spec.arg_index,
        }
    }
}

impl ConversionFields {
    fn apply_to(self, spec: &mut Spec) {
        spec.flags = self.flags;
        spec.width = self.width;
        spec.precision = self.precision;
    }
}

/// The cursor and mode committed by prior conversions in one format string.
#[derive(Debug, Default)]
struct FormatArgState {
    next: usize,
    saw_positional: bool,
    saw_sequential: bool,
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

/// Select and consume a conversion's dynamic fields and value. Complete and
/// incomplete conversions use this same owner so sequential cursors and
/// positional/sequential mixing cannot drift between their error paths.
fn consume_conversion_args<'a, O: ValueOps>(
    ops: &mut O,
    fields: &mut ConversionFields,
    args: &'a [O::Value],
    state: &mut FormatArgState,
) -> Result<&'a O::Value, CmdError> {
    let positional = fields.arg_index.is_some();
    if positional {
        if state.saw_sequential {
            return Err(CmdError::new(
                "cannot mix \"%\" and \"%n$\" conversion specifiers",
            ));
        }
        state.saw_positional = true;
    } else {
        if state.saw_positional {
            return Err(CmdError::new(
                "cannot mix \"%\" and \"%n$\" conversion specifiers",
            ));
        }
        state.saw_sequential = true;
    }
    let mut cur = match fields.arg_index {
        Some(n) => n
            .checked_sub(1)
            .ok_or_else(|| CmdError::new("\"%n$\" argument index out of range"))?,
        None => state.next,
    };
    // `*` width / `.*` precision take their values before the conversion
    // argument. A negative `*` width left-justifies.
    if fields.width_star {
        let w = ops.as_int(take_arg(args, &mut cur, positional)?)?;
        if w < 0 {
            fields.flags |= FmtFlags::MINUS;
        }
        let magnitude = usize::try_from(w.unsigned_abs()).unwrap_or(usize::MAX);
        if magnitude > MAX_RUNTIME_FIELD {
            return Err(value_too_large());
        }
        fields.width = Some(magnitude);
    }
    if fields.precision_star {
        let p = ops.as_int(take_arg(args, &mut cur, positional)?)?;
        fields.precision = Some(usize::try_from(p).unwrap_or(0).min(MAX_RUNTIME_FIELD));
    }
    let arg = take_arg(args, &mut cur, positional)?;
    if !positional {
        state.next = cur;
    }
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

/// Coerce a fixed-width integer conversion through Tcl 8.5+/9's bignum
/// magnitude seam. Tcl's `TclFormatInt` family first reduces arbitrary integer
/// objects modulo 2^64, then applies the conversion's selected width. Tcl 8.4
/// and Jim have no corresponding bignum format path, so they retain the legacy
/// wide-integer coercion and overflow error.
pub(crate) fn fixed_integer_value<O: ValueOps>(
    ops: &mut O,
    value: &O::Value,
    syntax: tcl_dialect::NumberSyntax,
) -> Result<i64, CmdError> {
    if !matches!(
        syntax,
        tcl_dialect::NumberSyntax::Tcl85 | tcl_dialect::NumberSyntax::Tcl90
    ) {
        return Ok(ops.as_int(value)?);
    }
    Ok(fixed_integer_magnitude(ops.integer_magnitude(
        value,
        Radix::Dec,
        syntax,
    )?))
}

/// Reduce an already obtained magnitude through the same fixed-width owner.
/// Checked adapters supply their original getter; this pure projection adds no
/// parsing, caching or object access.
#[must_use]
pub(crate) fn fixed_integer_magnitude(magnitude: IntegerMagnitude) -> i64 {
    let IntegerMagnitude { negative, digits } = magnitude;
    let magnitude = digits.bytes().fold(0_u64, |bits, digit| {
        debug_assert!(digit.is_ascii_digit());
        bits.wrapping_mul(10).wrapping_add(u64::from(digit - b'0'))
    });
    let bits = if negative {
        magnitude.wrapping_neg()
    } else {
        magnitude
    };
    i64::from_ne_bytes(bits.to_ne_bytes())
}

/// Reject a size modifier the release predates, naming the byte it reads as
/// the verb (`format %zd 1` on Tcl 8.6 is `bad field specifier "z"`).
fn reject_unknown_modifier(
    size: SizeModifier,
    syntax: tcl_dialect::NumberSyntax,
) -> Result<(), CmdError> {
    if size.accepted_under(syntax) {
        Ok(())
    } else {
        Err(CmdError::new(format!(
            "bad field specifier \"{}\"",
            char::from(size.lead_byte())
        )))
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
    if let Some(size) = spec.size {
        reject_unknown_modifier(size, syntax)?;
    }
    if verb == b'p' && syntax != tcl_dialect::NumberSyntax::Tcl90 {
        return Err(CmdError::new("bad field specifier \"p\""));
    }
    if spec.size.is_some_and(SizeModifier::is_big)
        && verb == b'u'
        && syntax != tcl_dialect::NumberSyntax::Tcl90
    {
        return Err(CmdError::with_error_code(
            "unsigned bignum format is invalid",
            "TCL FORMAT BADUNSIGNED",
        ));
    }
    if spec.size.is_some_and(SizeModifier::is_big)
        && matches!(verb, b'd' | b'i' | b'u' | b'x' | b'X' | b'o' | b'b')
    {
        return render_bignum_spec(ops, spec, arg, syntax);
    }
    match verb {
        b'd' | b'i' => {
            // The size modifier and the release pick the width; the low bits
            // are then read signed. See `tcl_syntax::format::integer_width`.
            let n = tcl_syntax::format::integer_width(spec.size, syntax)
                .signed(fixed_integer_value(ops, arg, syntax)?);
            let mut digits = int_digits(n, spec, syntax);
            // Tcl 9 `%#d` / `%#i` alternate form: a `0d` radix prefix on a
            // non-zero value (dropped for zero, like `%#x 0` → `0`). `%u` takes
            // no prefix. The sign and width are applied around it by
            // `pad_number`, so `%#d -42` → `-0d42`.
            if syntax.has_decimal_prefix() && spec.flags.contains(FmtFlags::HASH) && n != 0 {
                digits.insert_str(0, "0d");
            }
            Ok(pad_number(&digits, n < 0, spec))
        }
        b'u' => {
            // `%u` reads the same low bits *unsigned*: `format %u -1` is
            // 18446744073709551615 on 8.x and 4294967295 on 9.x, and
            // `format %hu 5000000000` is 61952 on every release.
            let u = tcl_syntax::format::integer_width(spec.size, syntax)
                .unsigned(fixed_integer_value(ops, arg, syntax)?);
            Ok(pad_unsigned_number(&uint_digits(u, spec, syntax), spec))
        }
        b'p' => {
            // `%p` is Tcl's pointer-style hexadecimal conversion, always with
            // a lowercase `0x` prefix. Its width is LP64's pointer width on
            // every target, wasm32 included; see `pointer_bits`.
            let u =
                tcl_syntax::format::pointer_bits(spec.size, fixed_integer_value(ops, arg, syntax)?);
            Ok(pad_unsigned_number(&pointer_digits(u, spec), spec))
        }
        b'x' | b'X' | b'o' | b'b' => {
            let u = tcl_syntax::format::integer_width(spec.size, syntax)
                .unsigned(fixed_integer_value(ops, arg, syntax)?);
            Ok(pad_unsigned_number(&based_digits(u, spec, syntax), spec))
        }
        b'c' => {
            let n = ops.as_c_int(arg)?;
            let ch = u32::try_from(n)
                .ok()
                .and_then(char::from_u32)
                .map_or_else(|| '\u{fffd}'.to_string(), |c| c.to_string());
            Ok(justify(&ch, spec))
        }
        b'f' | b'e' | b'E' | b'g' | b'G' => {
            let x = ops.as_double(arg)?;
            // C renders through `snprintf`, whose `int` count cannot reach a
            // precision of `INT_MAX` digits.
            if spec.precision.is_some_and(|p| p >= MAX_RUNTIME_FIELD) {
                return Err(value_too_large());
            }
            Ok(pad_number(
                &float_digits(x, spec),
                x.is_sign_negative(),
                spec,
            ))
        }
        b's' => {
            let mut s = ops.try_as_str(arg)?.to_string();
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

/// Render Tcl's `ll`/`L` bignum conversion without narrowing through `i64`.
fn render_bignum_spec<O: ValueOps>(
    ops: &mut O,
    spec: &Spec,
    arg: &O::Value,
    syntax: tcl_dialect::NumberSyntax,
) -> Result<String, CmdError> {
    let radix = match spec.verb {
        b'd' | b'i' | b'u' => Radix::Dec,
        b'o' => Radix::Oct,
        b'x' | b'X' => Radix::Hex,
        b'b' => Radix::Bin,
        _ => unreachable!("caller filters integer bignum conversions"),
    };
    let IntegerMagnitude {
        negative,
        mut digits,
    } = ops.integer_magnitude(arg, radix, syntax)?;
    if spec.verb == b'u' && negative {
        return Err(CmdError::with_error_code(
            "unsigned bignum format is invalid",
            "TCL FORMAT BADUNSIGNED",
        ));
    }
    if spec.verb == b'X' {
        digits.make_ascii_uppercase();
    }
    let nonzero = digits.bytes().any(|b| b != b'0');
    let prefix = alternate_prefix(spec, nonzero, syntax);
    digits = apply_radix_precision(digits, prefix, spec, syntax);
    Ok(pad_number(&digits, negative && spec.verb != b'u', spec))
}

/// Decimal digits (no sign) for an integer, honouring `.precision`.
fn int_digits(n: i64, spec: &Spec, syntax: tcl_dialect::NumberSyntax) -> String {
    apply_precision(n.unsigned_abs().to_string(), spec, syntax)
}

/// Decimal digits for an already-unsigned value, honouring `.precision`.
fn uint_digits(u: u64, spec: &Spec, syntax: tcl_dialect::NumberSyntax) -> String {
    apply_precision(u.to_string(), spec, syntax)
}

/// Digits for `x`/`X`/`o`/`b`, with the `#` alternate-form prefix.
fn based_digits(u: u64, spec: &Spec, syntax: tcl_dialect::NumberSyntax) -> String {
    // `u` already carries the conversion's width: the caller read the value's
    // low bits unsigned, which is why `%x` of a negative int prints its
    // two's-complement pattern, matching C's `format`.
    let body = match spec.verb {
        b'x' => format!("{u:x}"),
        b'X' => format!("{u:X}"),
        b'o' => format!("{u:o}"),
        _ => format!("{u:b}"),
    };
    let prefix = alternate_prefix(spec, u != 0, syntax);
    apply_radix_precision(body, prefix, spec, syntax)
}

/// Apply precision around a radix marker as Tcl's release grammar requires.
///
/// Tcl 8.x's legacy octal marker is a single leading `0`, which counts
/// toward integer precision. Tcl 9's two-byte `0o` marker, and the other
/// radix markers, remain outside the digit precision.
fn apply_radix_precision(
    digits: String,
    prefix: &str,
    spec: &Spec,
    syntax: tcl_dialect::NumberSyntax,
) -> String {
    if prefix == "0" {
        // The legacy marker is the zero digit itself for an octal zero. Do not
        // duplicate it, and retain it at `.0` where plain `%o` is empty on
        // Tcl 8.4.
        let marked = if digits == "0" {
            digits
        } else {
            format!("{prefix}{digits}")
        };
        let marked = apply_precision(marked, spec, syntax);
        if marked.is_empty() {
            "0".to_owned()
        } else {
            marked
        }
    } else {
        format!("{prefix}{}", apply_precision(digits, spec, syntax))
    }
}

/// Return the alternate-form radix marker for a conversion and release.
///
/// Tcl 8.5/8.6 retain the `0x`/`0X`/`0b` marker for zero. Tcl 8.4 and Jim
/// suppress it, as does Tcl 9, so this rule must be shared by the fixed-width
/// and bignum renderers.
fn alternate_prefix(spec: &Spec, nonzero: bool, syntax: tcl_dialect::NumberSyntax) -> &'static str {
    if !spec.flags.contains(FmtFlags::HASH) {
        return "";
    }
    let tcl9 = syntax.has_decimal_prefix();
    let tcl85 = matches!(syntax, tcl_dialect::NumberSyntax::Tcl85);
    let legacy_tcl = matches!(
        syntax,
        tcl_dialect::NumberSyntax::Tcl84 | tcl_dialect::NumberSyntax::Tcl85
    );
    match spec.verb {
        b'd' | b'i' if tcl9 && nonzero => "0d",
        b'x' if nonzero || tcl85 => "0x",
        b'X' if nonzero || tcl85 => {
            if tcl9 {
                "0x"
            } else {
                "0X"
            }
        }
        b'o' if nonzero || legacy_tcl => {
            if tcl9 {
                "0o"
            } else {
                "0"
            }
        }
        b'b' if nonzero || tcl85 => "0b",
        _ => "",
    }
}

/// Hexadecimal `%p` digits. Tcl preserves one zero digit even for an explicit
/// zero precision (`%.0p 0` → `0x0`), unlike the integer conversions.
fn pointer_digits(u: u64, spec: &Spec) -> String {
    let digits = format!("{u:x}");
    let digits = match spec.precision {
        Some(precision) if digits.len() < precision => {
            format!("{}{digits}", "0".repeat(precision - digits.len()))
        }
        _ => digits,
    };
    format!("0x{digits}")
}

/// Floating-point conversion verbs; Tcl does not accept C's `%F`.
fn is_float_verb(verb: u8) -> bool {
    matches!(verb, b'e' | b'E' | b'f' | b'g' | b'G')
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
/// C's `%g` at its default precision, with the sign: what `snprintf` writes
/// for a finite or infinite double (`1.23457e+08`, `-0`, `inf`).
pub(crate) fn c_default_g(x: f64) -> String {
    let spec = Spec {
        flags: FmtFlags::empty(),
        width: None,
        precision: None,
        verb: b'g',
        width_star: false,
        precision_star: false,
        arg_index: None,
        size: None,
    };
    let digits = float_digits(x, &spec);
    if x.is_sign_negative() {
        format!("-{digits}")
    } else {
        digits
    }
}

fn float_digits(x: f64, spec: &Spec) -> String {
    let prec = spec.precision.unwrap_or(6);
    let m = x.abs();
    if x.is_infinite() {
        return if matches!(spec.verb, b'E' | b'G') {
            "INF".to_owned()
        } else {
            "inf".to_owned()
        };
    }
    match spec.verb {
        b'f' | b'F' => {
            let mut out = format!("{m:.prec$}");
            if x.is_finite() && spec.flags.contains(FmtFlags::HASH) && !out.contains('.') {
                out.push('.');
            }
            out
        }
        b'e' => {
            let mut out = c_style_exp(&format!("{m:.prec$e}"));
            if x.is_finite()
                && spec.flags.contains(FmtFlags::HASH)
                && !out[..out.find('e').unwrap_or(out.len())].contains('.')
            {
                out.insert(out.find('e').unwrap_or(out.len()), '.');
            }
            out
        }
        b'E' => {
            let mut out = c_style_exp(&format!("{m:.prec$E}"));
            if x.is_finite()
                && spec.flags.contains(FmtFlags::HASH)
                && !out[..out.find('E').unwrap_or(out.len())].contains('.')
            {
                out.insert(out.find('E').unwrap_or(out.len()), '.');
            }
            out
        }
        // Shared `%g` owner also drives native legacy double strings.
        _ => tcl_syntax::number::format_general_float(
            m,
            prec,
            spec.flags.contains(FmtFlags::HASH),
            spec.verb == b'G',
        ),
    }
}

/// Left-pad `mag` with `0` up to `.precision` digits. Tcl 8.5+ and Tcl 9
/// integer conversions keep one zero digit when precision is zero, including
/// dynamic `.*` precision; Tcl 8.4 and Jim render that legacy case as empty.
fn apply_precision(mag: String, spec: &Spec, syntax: tcl_dialect::NumberSyntax) -> String {
    match spec.precision {
        Some(p) if mag.len() < p => format!("{}{mag}", "0".repeat(p - mag.len())),
        Some(0)
            if mag == "0"
                && matches!(
                    syntax,
                    tcl_dialect::NumberSyntax::Tcl85 | tcl_dialect::NumberSyntax::Tcl90
                ) =>
        {
            mag
        }
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
    let float = is_float_verb(spec.verb);
    // C's `printf` family always uses spaces for infinities and NaNs, even
    // when the `0` flag is present. The float renderer supplies these compact
    // spellings as the complete magnitude body.
    let nonfinite = float && matches!(body, "inf" | "INF" | "NaN" | "NAN");
    let zero = spec.flags.contains(FmtFlags::ZERO) && !nonfinite;
    if zero && !float && spec.precision.is_none() {
        let (prefix, rest) = numeric_prefix(body);
        format!("{sign}{prefix}{}{rest}", "0".repeat(pad))
    } else if spec.flags.contains(FmtFlags::MINUS) {
        format!("{sign}{body}{}", " ".repeat(pad))
    } else if zero && (spec.precision.is_none() || float) {
        // C ignores the `0` flag with an explicit precision for *integer*
        // conversions, but a float's precision is its fraction width, so `0`
        // still pads (`%08.2f 3.14` → `00003.14`).
        format!("{sign}{}{body}", "0".repeat(pad))
    } else {
        format!("{}{sign}{body}", " ".repeat(pad))
    }
}

/// Apply width to a fixed-width unsigned conversion.
///
/// C Tcl accepts `+` and space on fixed-width unsigned conversions, but does
/// not render either sign. Unbounded `ll`/`L` conversions use `pad_number`
/// directly because they retain those signs.
fn pad_unsigned_number(body: &str, spec: &Spec) -> String {
    let mut unsigned_spec = *spec;
    unsigned_spec.flags.remove(FmtFlags::PLUS | FmtFlags::SPACE);
    pad_number(body, false, &unsigned_spec)
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
    let pad_char = if spec.flags.contains(FmtFlags::ZERO) {
        '0'
    } else {
        ' '
    };
    let pad = pad_char.to_string().repeat(width - len);
    if spec.flags.contains(FmtFlags::MINUS) {
        format!("{s}{pad}")
    } else {
        format!("{pad}{s}")
    }
}

/// Split a numeric body's radix prefix from its digits so zero padding lands
/// after the sign and prefix, as in C Tcl (`%#08x` → `0x0000002a`).
fn numeric_prefix(body: &str) -> (&str, &str) {
    if body.len() >= 2
        && body.as_bytes()[0] == b'0'
        && matches!(body.as_bytes()[1], b'b' | b'd' | b'o' | b'x' | b'X')
    {
        body.split_at(2)
    } else {
        ("", body)
    }
}

#[cfg(test)]
mod tests {
    use tcl_dialect::NumberSyntax;
    use tcl_syntax::value::{ValueError, ValueOps};

    /// Strings as values, with `i64` integers: enough for fixed-width
    /// conversions, and free of any host-width type.
    struct StrOps;

    impl ValueOps for StrOps {
        type Value = String;
        fn new_str(&mut self, s: &str) -> String {
            s.to_owned()
        }
        fn new_int(&mut self, n: i64) -> String {
            n.to_string()
        }
        fn new_double(&mut self, f: f64) -> String {
            tcl_syntax::number::format_double(f)
        }
        fn new_bool(&mut self, b: bool) -> String {
            (if b { "1" } else { "0" }).to_owned()
        }
        fn new_list(&mut self, items: Vec<String>) -> String {
            items.join(" ")
        }
        fn as_bytes(&mut self, v: &String) -> std::rc::Rc<[u8]> {
            std::rc::Rc::from(v.as_bytes())
        }
        fn new_bytes(&mut self, bytes: &[u8]) -> String {
            std::str::from_utf8(bytes)
                .expect("Unicode-only fixture input")
                .to_owned()
        }
        fn as_int(&mut self, v: &String) -> Result<i64, ValueError> {
            v.parse().map_err(|_| ValueError::NotInteger(v.clone()))
        }
        fn as_double(&mut self, v: &String) -> Result<f64, ValueError> {
            v.parse().map_err(|_| ValueError::NotInteger(v.clone()))
        }
        fn as_bool(&mut self, _v: &String) -> Result<bool, ValueError> {
            Ok(false)
        }
        fn list_elements(&mut self, v: &String) -> Result<Vec<String>, ValueError> {
            Ok(v.split_whitespace().map(str::to_owned).collect())
        }
    }

    fn render(syntax: NumberSyntax, words: &[&str]) -> String {
        let args: Vec<String> = words.iter().map(|&w| w.to_owned()).collect();
        super::format_cmd_with_syntax(&mut StrOps, &args, syntax)
            .unwrap_or_else(|err| panic!("{words:?}: {err:?}"))
    }

    /// #2163: the renderer's ABI-dependent widths are LP64's whatever it is
    /// compiled for, so these hold on a wasm32 build as on the LP64 tclsh
    /// (8.6.18 / 9.0.4) the vectors come from.
    #[test]
    fn abi_dependent_widths_do_not_follow_the_compile_target_issue_2163() {
        for (words, expected) in [
            (&["%p", "4294967296"][..], "0x100000000"),
            (&["%Ip", "-1"], "0xffffffffffffffff"),
            (&["%hp", "65537"], "0x0"),
            (&["%zd", "4294967296"], "4294967296"),
            (&["%td", "-4294967297"], "-4294967297"),
            (&["%zx", "-1"], "ffffffffffffffff"),
            (&["%tu", "-1"], "18446744073709551615"),
            (&["%d", "4294967296"], "0"),
        ] {
            assert_eq!(render(NumberSyntax::Tcl90, words), expected, "{words:?}");
        }
        for (words, expected) in [
            (&["%d", "4294967296"][..], "4294967296"),
            (&["%u", "-1"], "18446744073709551615"),
            (&["%x", "-1"], "ffffffffffffffff"),
        ] {
            assert_eq!(render(NumberSyntax::Tcl85, words), expected, "{words:?}");
            assert_eq!(render(NumberSyntax::Tcl84, words), expected, "{words:?}");
        }
    }
}
