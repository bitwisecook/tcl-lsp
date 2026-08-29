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

//! `format` — format a string using printf-style conversion.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "format formatString ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

/// Constant-fold the
/// integer (`%d` / `%i` / `%u`), radix (`%x` / `%X` / `%o` / `%b`), character
/// (`%c`), float (`%f` / `%e` / `%E` / `%g` / `%G`) and string (`%s`)
/// conversions of `format`, with the printf flag / width / precision matrix.
/// The per-version behaviour was verified against `tclsh8.4`/`8.5`/`8.6`/`9.0`
/// during development and is pinned in this module's unit tests; the
/// differential harness (`tests/differential_fold.rs`) runs folds against the
/// live `tclsh9.0` reference.
/// `%b` (binary) *raises* before Tcl 8.6, so it is version-gated: the registry
/// passes the dialect's `version`, and `%b` folds only on 8.6+ (else bail).
/// A `ll`/`L` size modifier always bails too — it is unmodelled (its
/// "take without truncation" width isn't handled by any `render_*` helper,
/// and `%llu` specifically *raises* below Tcl 9.0: `tclsh8.6 format %llu 5`
/// → "unsigned bignum format is invalid", empirically verified), so folding
/// it as a plain verb would be unsound.
/// Only other size modifiers and `*`/positional specs remain unfolded.
///
/// `args[0]` is the format string (ASCII-restricted — we byte-index it for
/// the `%` scan, so a multi-byte char would be mis-sliced); `args[1..]` are
/// the values.  Soundness first — fold only the dialect-invariant subset and
/// bail (return `None`, leaving the call unfolded) on everything else, since a
/// wrong fold is a false-positive optimisation.  Each conversion's exact
/// constraints live on its `render_*` helper; in summary:
///
/// * `%d` / `%i` / `%x` / `%X` / `%o` / `%b` interpret the integer argument
///   **per Tcl version** ([`parse_format_int`]): a leading zero is octal on
///   8.x but decimal on 9.0, and 9.0 wraps to signed 32-bit while 8.x keeps the
///   full 64-bit value — so a radix conversion also renders a negative as the
///   dialect-width two's complement (`%x -1` → `ffffffff` on 9.0,
///   `ffffffffffffffff` on 8.6).  An unversioned dialect folds only the
///   invariant plain-decimal i32 subset (a radix negative bails).  `%c` uses
///   the invariant [`parse_decimal_arg`] (a printable-ASCII codepoint).  The
///   `#` alternate form is sound only as the lowercase `0x` / `0b` on a
///   non-zero `%x` / `%b`.
/// * `%s` honours `-` / width / precision; the numeric flags bail (Tcl
///   ignores them on strings), and width/precision over a non-ASCII value
///   bails (the character count diverges 8.x UTF-16 units ↔ 9.0 codepoints).
/// * the float conversions parse a double over the same subset as `string is
///   double` ([`parse_float_arg`]) and build on Rust's float formatter, with
///   `%e`/`%g`'s exponent / shortest-form rendering reshaped to C's by
///   [`fmt_sci`] / [`fmt_general`]; a non-finite value and the `#` form bail.
///   See [`Conversion::render_float`].
/// * `%b` (binary) routes through [`Conversion::render_radix`] like `%x`
///   (non-negative, `0b` for `%#b`), but only on Tcl 8.6+ — it raises in
///   8.4/8.5, so `fold_format` bails there.  A `ll`/`L` size modifier, `*`
///   (arg-driven) width / precision, positional `%n$`, and any field over
///   [`MAX_FIELD`] all bail.
/// * A bare trailing `%` (an incomplete conversion, which Tcl raises on) and
///   too few arguments both bail; extra arguments are ignored (matching Tcl).
///
/// KNOWN GAP (not fixed here — needs a `tcl-syntax` change, out of this
/// file's scope): a single-letter size modifier (`h`, `l`, `z`, `t`, `j`,
/// `q`) is parsed and silently discarded by [`parse_spec`] rather than
/// exposed on [`Spec`], so it is *not* modelled or bailed on here — `%hd`
/// folds as plain `%d`.  This is unsound in general: `h` truncates to a
/// 16-bit value before conversion regardless of platform or dialect
/// (empirically verified: `tclsh8.6 format %hd 5000000000` → `-3584`, but
/// `tclsh8.6 format %d 5000000000` → `5000000000` on a wordSize-8 build —
/// this file's `fold_format` would currently (wrongly) fold `%hd
/// 5000000000` to `"5000000000"`).  `l`/`z`/`t`/`j`/`q` often coincide with
/// the no-modifier width on common platforms/dialects but are not
/// guaranteed to.  Fixing this soundly requires `tcl-syntax::format::Spec`
/// to report *which* (if any) single-letter modifier was present, not just
/// the `ll`/`L` case it currently exposes via `big`.
fn fold_format(args: &[&str], version: Option<TclVersion>) -> Option<String> {
    let (fmt, vals) = args.split_first()?;
    if !fmt.is_ascii() {
        return None;
    }
    let fmt = fmt.as_bytes();
    let mut out = String::new();
    let mut ai = 0usize;
    let mut i = 0usize;
    while i < fmt.len() {
        if fmt[i] != b'%' {
            // `fmt` is ASCII, so each byte is one char.
            out.push(char::from(fmt[i]));
            i += 1;
            continue;
        }
        i += 1;
        match fmt.get(i) {
            None => return None, // bare trailing `%` — incomplete spec
            Some(&b'%') => {
                out.push('%'); // `%%` → literal percent
                i += 1;
                continue;
            }
            _ => {}
        }
        let conv = Conversion::parse(fmt, &mut i)?;
        // `%b` (binary) *raises* before Tcl 8.6 — fold it only when the dialect
        // is 8.6+; every other conversion here is version-invariant.
        if conv.verb == b'b' && version.is_none_or(|v| v < TclVersion::V8_6) {
            return None;
        }
        let value = vals.get(ai)?;
        ai += 1;
        out.push_str(&conv.render(value, version)?);
    }
    Some(out)
}

/// The specifier *grammar* (flags / width / `.precision` / verb parsing) lives
/// in the shared `tcl-syntax` crate so the WASM runtime reuses it; the
/// version-aware renderers below are this const-folder's own.
use tcl_syntax::format::{FmtFlags, Spec, parse_spec};
use tcl_dialect::model::{SpecSurface};

/// A single parsed `%…` conversion — the render input. Matches
/// [`tcl_syntax::format::Spec`]; kept local so the version-aware `render*`
/// methods can hang off it (the orphan rule forbids `impl` on the shared type).
struct Conversion {
    flags: FmtFlags,
    width: Option<usize>,
    precision: Option<usize>,
    verb: u8,
}

/// The base / case of a radix conversion (`%x` / `%X` / `%o` / `%b`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Radix {
    HexLower,
    HexUpper,
    Octal,
    Binary,
}

/// The notation of a float conversion: `%f` (fixed), `%e` (scientific), `%g`
/// (general — the shorter of the two with trailing zeros stripped).
#[derive(Clone, Copy)]
enum FloatKind {
    Fixed,
    Scientific,
    General,
}

impl Conversion {
    /// Parse one conversion via the shared [`tcl_syntax::format`] grammar,
    /// advancing `i` past the verb.
    fn parse(fmt: &[u8], i: &mut usize) -> Option<Self> {
        let Spec {
            flags,
            width,
            precision,
            verb,
            width_star,
            precision_star,
            arg_index,
            // A `ll`/`L` size modifier ("take without truncation"): unmodelled
            // by every `render_*` helper below, and version-relevant on its
            // own — `%llu` *raises* before Tcl 9.0 (empirically verified:
            // `tclsh8.6 format %llu 5` → "unsigned bignum format is
            // invalid"; 9.0 accepts it) while `%lld`/`%llx`/... skip the
            // dialect's normal wrap width on 9.0+. Bailed on below, alongside
            // the other unmodelled forms.
            big,
        } = parse_spec(fmt, i)?;
        // Decline to fold the unmodelled specifier forms: arg-driven `*` width /
        // `.*` precision, positional `%n$` selectors, and a `ll`/`L` size
        // modifier. `fold_format` consumes the value list left-to-right, so a
        // positional spec would fold to the wrong argument (`format {%2$d}
        // 10 20` is Tcl `20`, but sequentially `10`) — and a *mixed*
        // positional/sequential format is a Tcl error. Bailing keeps the
        // constant folder from rewriting either to a wrong value; the runtime
        // `format` handles them.
        if width_star || precision_star || arg_index.is_some() || big {
            return None;
        }
        Some(Self {
            flags,
            width,
            precision,
            verb,
        })
    }

    /// Render this conversion for `value`, or bail (`None`) for an unmodelled
    /// verb or an out-of-subset argument.
    fn render(&self, value: &str, version: Option<TclVersion>) -> Option<String> {
        match self.verb {
            b'd' | b'i' => self.render_int(value, version),
            b'x' => self.render_radix(value, Radix::HexLower, version),
            b'X' => self.render_radix(value, Radix::HexUpper, version),
            b'o' => self.render_radix(value, Radix::Octal, version),
            b'c' => self.render_char(value),
            b'f' => self.render_float(value, FloatKind::Fixed, false),
            b'e' => self.render_float(value, FloatKind::Scientific, false),
            b'E' => self.render_float(value, FloatKind::Scientific, true),
            b'g' => self.render_float(value, FloatKind::General, false),
            b'G' => self.render_float(value, FloatKind::General, true),
            b'u' => self.render_unsigned(value, version),
            b'b' => self.render_radix(value, Radix::Binary, version),
            b's' => self.render_str(value),
            // Unmodelled verbs (`%a`/`%A`/`%p`) and any invalid byte —
            // `F` is not a real Tcl conversion (empirically verified:
            // `tclsh8.6 format %F 3.14` → `bad field specifier "F"`) — bail.
            _ => None,
        }
    }

    /// Render `%u` (unsigned decimal) — the **decimal** of the integer
    /// argument's unsigned bit pattern at the dialect's width (like
    /// [`Self::render_radix`], sharing [`parse_format_int`]): 9.0 wraps to a
    /// 32-bit unsigned (`%u -1` → `4294967295`, `%u 4294967296` → `0`), 8.x to
    /// a 64-bit unsigned (`%u -1` → `18446744073709551615`).  An unversioned
    /// dialect keeps the non-negative `[0, 2³²-1]` subset.  `%u` carries no
    /// sign, so `+`/space don't apply; `#` bails.
    fn render_unsigned(&self, value: &str, version: Option<TclVersion>) -> Option<String> {
        if self.flags.contains(FmtFlags::HASH) {
            return None;
        }
        let signed = parse_format_int(value, version)?;
        let n: u64 = match version {
            Some(TclVersion::V9_0 | TclVersion::V9_1) => u64::from(u32::from_ne_bytes(
                i32::try_from(signed).ok()?.to_ne_bytes(),
            )),
            Some(_) => u64::from_ne_bytes(signed.to_ne_bytes()),
            None => u64::try_from(signed).ok()?, // unversioned: non-negative only
        };
        let mut digits = n.to_string();
        if let Some(p) = self.precision
            && digits.len() < p
        {
            digits = "0".repeat(p - digits.len()) + &digits;
        }
        Some(self.pad("", &digits, self.int_zero_pad()))
    }

    /// Render a float conversion — `%f` (fixed; `%F` is *not* a valid Tcl
    /// conversion on any version — empirically verified: `tclsh8.6 format %F
    /// 3.14` → `bad field specifier "F"`), `%e`/`%E` (scientific), or
    /// `%g`/`%G` (general).  All build on Rust's float formatter, which is
    /// byte-identical to C/Tcl on the exact binary value — same
    /// round-half-to-even (verified differentially: `2.5`→`2`, `0.125`→`0.12`,
    /// `2.675`→`2.67`, `99.995`→`100.00`) — with the exponent / shortest-form
    /// rendering reshaped to C's by [`fmt_sci`] / [`fmt_general`].  Precision
    /// defaults to 6 (the C default).  Unlike `%d`, the `0` flag zero-pads
    /// even with a precision.  A non-finite value (`Inf`/`NaN` — spelling is
    /// version-edgy) and the `#` alternate form (it suppresses `%g`'s
    /// trailing-zero stripping) both bail.
    fn render_float(&self, value: &str, kind: FloatKind, upper: bool) -> Option<String> {
        if self.flags.contains(FmtFlags::HASH) {
            return None;
        }
        let v = parse_float_arg(value)?;
        if !v.is_finite() {
            return None;
        }
        let prec = self.precision.unwrap_or(6);
        let mag = match kind {
            FloatKind::Fixed => format!("{:.*}", prec, v.abs()),
            FloatKind::Scientific => fmt_sci(v.abs(), prec, upper),
            FloatKind::General => fmt_general(v.abs(), prec, upper),
        };
        let sign = if v.is_sign_negative() {
            "-"
        } else if self.flags.contains(FmtFlags::PLUS) {
            "+"
        } else if self.flags.contains(FmtFlags::SPACE) {
            " "
        } else {
            ""
        };
        Some(self.pad(sign, &mag, self.flags.contains(FmtFlags::ZERO)))
    }

    /// Render `%c` (codepoint → character) for a printable-ASCII codepoint
    /// (space..=`~`).  That range is byte-identical across every Tcl version;
    /// a codepoint ≥ 128 diverges (8.6 emits a byte, 9.0 raises), control
    /// chars / DEL are unsafe to splice into source, and a negative codepoint
    /// raises — all bail.  Only the `-` flag + width apply; precision and the
    /// numeric flags bail.
    fn render_char(&self, value: &str) -> Option<String> {
        if self.precision.is_some()
            || self
                .flags
                .intersects(FmtFlags::PLUS | FmtFlags::SPACE | FmtFlags::ZERO | FmtFlags::HASH)
        {
            return None;
        }
        let n = parse_decimal_arg(value)?;
        if !(32..=126).contains(&n) {
            return None;
        }
        let ch = char::from(u8::try_from(n).ok()?);
        Some(self.pad("", &ch.to_string(), false))
    }

    /// Render `%d` / `%i`.  The integer *value* is version-aware
    /// ([`parse_format_int`]): a leading-zero argument is octal on 8.x but
    /// decimal on 9.0, and 9.0 wraps to a signed 32-bit int while 8.x keeps the
    /// full 64-bit value — so `%d 010` → `8` on 8.6 but `10` on 9.0, and
    /// `%d 2147483648` → that value on 8.x but `-2147483648` on 9.0.  An
    /// unversioned dialect folds only the invariant plain-decimal i32 subset.
    fn render_int(&self, value: &str, version: Option<TclVersion>) -> Option<String> {
        if self.flags.contains(FmtFlags::HASH) {
            return None; // `%#d` diverges 8.6 (`42`) ↔ 9.0 (`0d42`)
        }
        let n = parse_format_int(value, version)?;
        let mut digits = n.unsigned_abs().to_string();
        if let Some(p) = self.precision
            && digits.len() < p
        {
            digits = "0".repeat(p - digits.len()) + &digits;
        }
        let sign = if n < 0 {
            "-"
        } else if self.flags.contains(FmtFlags::PLUS) {
            "+"
        } else if self.flags.contains(FmtFlags::SPACE) {
            " "
        } else {
            ""
        };
        Some(self.pad(sign, &digits, self.int_zero_pad()))
    }

    /// Render `%x` / `%X` / `%o` / `%b`.  The integer *value* is version-aware
    /// ([`parse_format_int`], like `%d`), and its two's-complement **bit width**
    /// is the dialect's: 9.0 wraps to 32 bits, 8.x to 64 bits — so `%x -1` is
    /// `ffffffff` on 9.0 but `ffffffffffffffff` on 8.6, and `%x 010` is `a`
    /// (decimal 10) on 9.0 but `8` (octal) on 8.x.  An **unversioned** dialect
    /// keeps the conservative non-negative i32 subset (a negative bails, its
    /// width being divergent).  `+` / space don't apply to a radix conversion
    /// (bail).  The `#` alternate form is sound only as the lowercase `0x` /
    /// `0b` prefix on a non-zero value (`%#X` is `0XFF` on 8.6 but `0xFF` on
    /// 9.0, `%#o` is `010` vs `0o10`, `%#x 0` / `%#b 0` are `0x0`/`0b0` vs `0`).
    fn render_radix(
        &self,
        value: &str,
        radix: Radix,
        version: Option<TclVersion>,
    ) -> Option<String> {
        if self.flags.intersects(FmtFlags::PLUS | FmtFlags::SPACE) {
            return None;
        }
        let signed = parse_format_int(value, version)?;
        // The two's-complement bit pattern at the dialect's width.
        let n: u64 = match version {
            Some(TclVersion::V9_0 | TclVersion::V9_1) => u64::from(u32::from_ne_bytes(
                i32::try_from(signed).ok()?.to_ne_bytes(),
            )),
            Some(_) => u64::from_ne_bytes(signed.to_ne_bytes()),
            None => u64::try_from(signed).ok()?, // unversioned: non-negative only
        };
        let mut digits = match radix {
            Radix::HexLower => format!("{n:x}"),
            Radix::HexUpper => format!("{n:X}"),
            Radix::Octal => format!("{n:o}"),
            Radix::Binary => format!("{n:b}"),
        };
        let prefix = if self.flags.contains(FmtFlags::HASH) {
            // The `#` prefix is sound only as a lowercase `0x` / `0b` on a
            // non-zero value; `#X` / `#o` / `#x 0` / `#b 0` diverge.
            match radix {
                Radix::HexLower if n != 0 => "0x",
                Radix::Binary if n != 0 => "0b",
                _ => return None,
            }
        } else {
            ""
        };
        if let Some(p) = self.precision
            && digits.len() < p
        {
            digits = "0".repeat(p - digits.len()) + &digits;
        }
        Some(self.pad(prefix, &digits, self.int_zero_pad()))
    }

    fn render_str(&self, value: &str) -> Option<String> {
        // `+` / space / `0` / `#` don't apply to `%s`; Tcl ignores them, but
        // bailing keeps the modelled surface minimal and is never wrong.
        if self
            .flags
            .intersects(FmtFlags::PLUS | FmtFlags::SPACE | FmtFlags::ZERO | FmtFlags::HASH)
        {
            return None;
        }
        // Width / precision count *characters*; that count diverges for
        // non-ASCII across Tcl 8.x (UTF-16 units) and 9.0 (codepoints).
        if (self.width.is_some() || self.precision.is_some()) && !value.is_ascii() {
            return None;
        }
        let s: String = match self.precision {
            Some(p) => value.chars().take(p).collect(),
            None => value.to_owned(),
        };
        let width = self.width.unwrap_or(0);
        let len = s.chars().count();
        if len >= width {
            return Some(s);
        }
        let pad = " ".repeat(width - len);
        Some(if self.flags.contains(FmtFlags::MINUS) {
            s + &pad
        } else {
            pad + &s
        })
    }

    /// Width-pad a `prefix` + `digits` numeric body (the `prefix` is a sign
    /// `-`/`+`/space for `%d`/`%f`, or a `0x` radix marker for `%#x`):
    /// zero-pad between the prefix and digits when `zero_pad` is set,
    /// left-justify with spaces under `-`, else right-justify with spaces.
    /// (The caller decides `zero_pad`: for the integer conversions a
    /// precision suppresses the `0` flag, per C/Tcl, but for `%f` it does not.)
    fn pad(&self, prefix: &str, digits: &str, zero_pad: bool) -> String {
        let width = self.width.unwrap_or(0);
        let body_len = prefix.len() + digits.len();
        if body_len >= width {
            return format!("{prefix}{digits}");
        }
        let fill = width - body_len;
        if self.flags.contains(FmtFlags::MINUS) {
            format!("{prefix}{digits}{}", " ".repeat(fill))
        } else if zero_pad {
            format!("{prefix}{}{digits}", "0".repeat(fill))
        } else {
            format!("{}{prefix}{digits}", " ".repeat(fill))
        }
    }

    /// Whether the `0` flag should zero-pad for an *integer* conversion: set,
    /// but suppressed by a precision (C/Tcl rule for `%d`/`%x`/`%o`).
    fn int_zero_pad(&self) -> bool {
        self.flags.contains(FmtFlags::ZERO) && self.precision.is_none()
    }
}

/// Parse a `%d` / `%i` argument as a **dialect-invariant** decimal integer:
/// optional `+`/`-` sign, ASCII digits only, no leading zero (other than a
/// lone `0`), and within the signed 32-bit range — the subset where every Tcl
/// 8.4 → 9.0 agrees.  Outside it the interpretations diverge: a leading zero
/// is octal in 8.x but decimal in 9.0 (`%d 010` → `8` vs `10`), and a value
/// past 2³¹ wraps to 32 bits in 9.0 but not 8.6 (`%d 2147483648` →
/// `-2147483648` vs `2147483648`).  Hex / octal / binary prefixes are rejected
/// too (Rust's parser declines them — a sound miss).  Returns `None` for
/// anything outside the invariant subset.
fn parse_decimal_arg(value: &str) -> Option<i64> {
    let s = value.trim();
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return None; // leading zero → octal in 8.x, decimal in 9.0
    }
    let n: i64 = s.parse().ok()?;
    if !(-2_147_483_648..=2_147_483_647).contains(&n) {
        return None; // outside the 32-bit range 8.x ↔ 9.0 agree on
    }
    Some(n)
}

/// Parse a `%d`/`%i` argument as the **version-specific** signed value to
/// format (as an `i64`).  Restricted to a plain decimal / leading-zero form
/// (no `0x`/`0o`/`0b` prefix — those bail), but the leading-zero and
/// out-of-range semantics follow the dialect:
///
/// * `None` (unversioned): the invariant subset — plain decimal, no leading
///   zero, signed 32-bit ([`parse_decimal_arg`]).
/// * `9.0`: a leading zero is decimal, and any magnitude wraps to a signed
///   32-bit int (`010` → 10, `2147483648` → -2147483648, `5000000000` →
///   705032704).
/// * `8.4`/`8.5`/`8.6`: a leading zero is octal (a digit ≥ 8 raises → bail),
///   and the value must fit a signed 64-bit int — beyond it the releases
///   diverge (8.5 mis-converts 2⁶³), so bail.
fn parse_format_int(value: &str, version: Option<TclVersion>) -> Option<i64> {
    match version {
        None => parse_decimal_arg(value),
        Some(TclVersion::V9_0 | TclVersion::V9_1) => {
            let big = parse_int_i128(value, false)?;
            // Reduce mod 2³² then reinterpret as a signed 32-bit int (the wrap).
            let wrapped = u32::try_from(big.rem_euclid(4_294_967_296)).ok()?;
            Some(i64::from(i32::from_ne_bytes(wrapped.to_ne_bytes())))
        }
        Some(_) => i64::try_from(parse_int_i128(value, true)?).ok(),
    }
}

/// Parse a plain decimal / leading-zero integer argument to `i128`.  With
/// `octal_leading_zero` (8.x) a leading-zero number is octal (a digit ≥ 8 →
/// `None`, as Tcl raises); otherwise (9.0) it is decimal.  A `0x`/`0o`/`0b`
/// prefix or any non-decimal-digit form bails.
fn parse_int_i128(value: &str, octal_leading_zero: bool) -> Option<i128> {
    if !value.is_ascii() {
        return None;
    }
    let t = value.trim();
    let (neg, body) = match t.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let b = body.as_bytes();
    if b.is_empty() {
        return None;
    }
    if b[0] == b'0' && b.len() >= 2 && matches!(b[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B') {
        return None; // a prefixed form — a further follow-up
    }
    if !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mag: i128 = if b.len() > 1 && b[0] == b'0' && octal_leading_zero {
        if b.iter().any(|&d| d > b'7') {
            return None; // invalid octal (an 8 or 9) — Tcl raises
        }
        i128::from_str_radix(body, 8).ok()?
    } else {
        body.parse::<i128>().ok()?
    };
    Some(if neg { -mag } else { mag })
}

/// Parse a `%f` argument as a double.  ASCII-gated (so the `.trim()`
/// whitespace set matches Tcl's `Tcl_GetDouble`), then `f64::from_str` —
/// which agrees with Tcl's parse on the decimal / scientific forms (verified
/// in `string is double`).  The version-divergent / Rust-rejected forms
/// (a `0x`/`0o`/`0b` prefix, a `_` separator, a `nan(payload)`) all fail
/// `from_str` and bail here; the caller additionally bails on a non-finite
/// result.
fn parse_float_arg(value: &str) -> Option<f64> {
    if !value.is_ascii() {
        return None;
    }
    value.trim().parse::<f64>().ok()
}

/// `%e` / `%E` magnitude: Rust's `{:.prec$e}`, with the exponent reshaped to
/// C's form — a sign and at least two digits (`3.140000e+04`, not `3.14e4`).
/// The mantissa matches C's rounding (same round-half-to-even).
fn fmt_sci(v: f64, prec: usize, upper: bool) -> String {
    let raw = format!("{v:.prec$e}");
    let (mant, exp) = raw.split_once('e').unwrap_or((raw.as_str(), "0"));
    let e: i32 = exp.parse().unwrap_or(0);
    let marker = if upper { 'E' } else { 'e' };
    let sign = if e < 0 { '-' } else { '+' };
    format!("{mant}{marker}{sign}{:02}", e.abs())
}

/// `%g` / `%G` magnitude (C semantics): with effective precision `P = max(p,
/// 1)`, pick `%f` when the decimal exponent `X` satisfies `-4 ≤ X < P` (with
/// precision `P-1-X`) else `%e` (with precision `P-1`), then strip trailing
/// zeros — and a trailing `.` — from the mantissa (the default, non-`#`,
/// behaviour; `%#g` bails upstream).
fn fmt_general(v: f64, prec: usize, upper: bool) -> String {
    let p = i32::try_from(prec.max(1)).unwrap_or(i32::MAX);
    let exp: i32 = format!("{:.*e}", prec.max(1) - 1, v)
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0);
    let mut body = if (-4..p).contains(&exp) {
        let fprec = usize::try_from(p - 1 - exp).unwrap_or(0);
        format!("{v:.fprec$}")
    } else {
        fmt_sci(v, prec.max(1) - 1, upper)
    };
    // Strip trailing zeros from the mantissa only, leaving any exponent intact.
    let marker = if upper { 'E' } else { 'e' };
    if let Some(epos) = body.find(marker) {
        let (mant, exp) = body.split_at(epos);
        if mant.contains('.') {
            body = format!("{}{exp}", mant.trim_end_matches('0').trim_end_matches('.'));
        }
    } else if body.contains('.') {
        body = body.trim_end_matches('0').trim_end_matches('.').to_owned();
    }
    body
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "format",
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        byte_array_effect: ByteArrayEffect::Coerces,
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::PURE
            | Traits::CSE_CANDIDATE,
        // Index 0 is the %-string (the §6 argument-DSL rung: `%b` is
        // 8.6+, `%ll…u` is 9.0+).
        arg_roles: &[(0, ArgRole::FormatString)],
        // The family of this command's format string; the
        // `ArgRole::FormatString` role above locates the word. Together
        // they are the whole registry answer, so the LSP's format
        // highlighting and inlay hints name no command (#1185).
        format_string_type: Some(FormatType::Sprintf),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        const_fold_versioned: Some(fold_format),
        hover: Some(HoverSnippet {
            summary: "Format a string in the style of C's sprintf.",
            synopsis: &["format formatString ?arg arg ...?"],
            snippet: "Scans formatString left to right, copying text through unchanged except where a % introduces a conversion specifier that consumes the next arg (or, with an %n$ position, an explicit one — once any specifier in the string is positional, every specifier must be). A specifier is %[n$][flags][width][.precision][size]verb: the flags are - (left-justify instead of the default right-justify), + (always show a sign), space (a leading space on a non-negative number), 0 (zero-pad instead of space-pad), and # (alternate form: on every version it forces a decimal point on a float and preserves %g's trailing zeros; for the integer verbs, before Tcl 9.0 it adds a leading 0 on octal or 0x/0X on hex (matching the verb's case) or, from Tcl 8.6, 0b on binary, while from Tcl 9.0 octal instead gets an 0o prefix, hex always gets a lowercase 0x prefix, and decimal gains a new 0d prefix). width and precision may each be a literal number or * to take the value from the next argument; precision means digits after the point for e/f, total significant digits for g, a minimum digit count (zero-padded) for the integer verbs, and a maximum character count for s. Conversion verbs: d/i (signed decimal), u (unsigned decimal), o (unsigned octal), x/X (unsigned hex), b (unsigned binary — Tcl 8.6 and later only), c (integer codepoint to character), s (string, unconverted), f (fixed-point), e/E (scientific notation), g/G (the shorter of f or e, with trailing zeros stripped), and %% for a literal percent. Tcl 9.0 and later also accept a/A (hexadecimal floating-point) and p (shorthand for 0x%zx). An optional size modifier before the verb (h or l; also ll from Tcl 8.5, and z, t, L, j, or q from Tcl 9.0) controls the integer conversion's truncation width but is ignored for the float verbs and for %c/%s. Too few arguments, or a malformed specifier, raises an error; unused trailing arguments are ignored.",
            source: "Tcl man page format.n",
            examples: "format {Hello, %s! You are %d years old.} $name $age\nformat %05.2f $pi\nformat {%#x} 255\nformat {%-10s|} $label",
            return_value: "The formatted string.",
        }),
        forms: FORMS,
        ..CommandSpec::CLOSED_REFERENTIALLY_TRANSPARENT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_format;
    use crate::hooks::TclVersion;

    /// The version-invariant conversions behave the same on every dialect;
    /// drive them under 9.0 (the version-gated `%b` has its own test).
    fn f(args: &[&str]) -> Option<String> {
        fold_format(args, Some(TclVersion::V9_0))
    }

    #[test]
    fn format_folds_plain_s_d_percent_subset() {
        assert_eq!(f(&["hello"]).as_deref(), Some("hello"));
        assert_eq!(f(&["%s", "world"]).as_deref(), Some("world"));
        assert_eq!(f(&["%d", "42"]).as_deref(), Some("42"));
        assert_eq!(f(&["%d", " -7 "]).as_deref(), Some("-7"));
        assert_eq!(f(&["v=%s n=%d", "x", "3"]).as_deref(), Some("v=x n=3"));
        assert_eq!(f(&["100%%"]).as_deref(), Some("100%"));
        // `%d` with a non-integer arg bails (Tcl errors at runtime).
        assert_eq!(f(&["%d", "hi"]), None);
        // Too few args bails.
        assert_eq!(f(&["%s %s", "only"]), None);
    }

    #[test]
    fn format_does_not_fold_positional_specifiers() {
        // The folder consumes args left-to-right, so a positional `%n$` selector
        // must bail rather than fold to the wrong argument — `format {%2$d}
        // 10 20` is `20` in tclsh, never the sequential `10`. A *mixed*
        // positional/sequential format is a tclsh error, so it must not fold
        // either. (Verified against tclsh 9.0.)
        assert_eq!(f(&["%2$d", "10", "20"]), None);
        assert_eq!(f(&["%1$s", "x"]), None);
        assert_eq!(f(&["%2$d %d", "10", "20"]), None); // mixed → tclsh errors
    }

    #[test]
    fn format_folds_integer_flag_width_precision() {
        // width / justify / zero-pad / sign — pinned against tclsh8.6 + 9.0.
        assert_eq!(f(&["%5d", "42"]).as_deref(), Some("   42"));
        assert_eq!(f(&["%-5d", "42"]).as_deref(), Some("42   "));
        assert_eq!(f(&["%05d", "7"]).as_deref(), Some("00007"));
        assert_eq!(f(&["%05d", "-7"]).as_deref(), Some("-0007"));
        assert_eq!(f(&["%+d", "42"]).as_deref(), Some("+42"));
        assert_eq!(f(&["% d", "42"]).as_deref(), Some(" 42"));
        assert_eq!(f(&["% d", "-7"]).as_deref(), Some("-7"));
        assert_eq!(f(&["%+05d", "42"]).as_deref(), Some("+0042"));
        assert_eq!(f(&["%5d", "-42"]).as_deref(), Some("  -42"));
        assert_eq!(f(&["%i", "42"]).as_deref(), Some("42")); // %i alias
        // precision = minimum digits; the `0` flag is suppressed by precision.
        assert_eq!(f(&["%.5d", "42"]).as_deref(), Some("00042"));
        assert_eq!(f(&["%.3d", "-4"]).as_deref(), Some("-004"));
        assert_eq!(f(&["%5.3d", "42"]).as_deref(), Some("  042"));
        assert_eq!(f(&["%05.3d", "42"]).as_deref(), Some("  042"));
        assert_eq!(f(&["%.0d", "0"]).as_deref(), Some("0"));
    }

    #[test]
    fn format_folds_string_width_precision() {
        assert_eq!(f(&["%10s", "hi"]).as_deref(), Some("        hi"));
        assert_eq!(f(&["%-10s", "hi"]).as_deref(), Some("hi        "));
        assert_eq!(f(&["%.3s", "hello"]).as_deref(), Some("hel"));
        assert_eq!(f(&["%5.3s", "hello"]).as_deref(), Some("  hel"));
        assert_eq!(f(&["%.0s", "hi"]).as_deref(), Some(""));
    }

    #[test]
    fn format_bails_on_version_divergent_and_unmodelled() {
        // `%#d` is `0d42` on Tcl 9 but `42` on 8.6 — no sound fold.
        assert_eq!(f(&["%#d", "42"]), None);
        // The 32-bit boundary folds; the version-specific value-interpretation
        // (`f` runs under 9.0: leading-zero decimal, 32-bit wrap) is covered by
        // `format_d_value_is_version_aware`.
        assert_eq!(f(&["%d", "2147483647"]).as_deref(), Some("2147483647"));
        assert_eq!(f(&["%d", "-2147483648"]).as_deref(), Some("-2147483648"));
        // `0x`/`0o`/`0b` prefixes bail (a further follow-up).
        assert_eq!(f(&["%d", "0x10"]), None);
        // %u (the version-specific value-interp is in
        // `format_unsigned_value_is_version_aware`; `f` runs under 9.0).
        assert_eq!(f(&["%u", "42"]).as_deref(), Some("42"));
        assert_eq!(f(&["%05u", "42"]).as_deref(), Some("00042"));
        assert_eq!(f(&["%.3u", "42"]).as_deref(), Some("042"));
        // Float verbs with a non-finite value bail (Inf/NaN spelling is edgy);
        // the `#` alternate form bails (it changes %g trailing-zero stripping).
        assert_eq!(f(&["%f", "inf"]), None);
        assert_eq!(f(&["%e", "nan"]), None);
        assert_eq!(f(&["%#f", "3.14"]), None);
        assert_eq!(f(&["%#g", "3.14"]), None);
        // Numeric flags don't apply to `%s` → bail.
        assert_eq!(f(&["%05s", "hi"]), None);
        assert_eq!(f(&["%+s", "hi"]), None);
        // Non-ASCII value under a width/precision bails (char-count diverges).
        assert_eq!(f(&["%5s", "café"]), None);
        // Arg-driven width / precision and over-cap fields bail.
        assert_eq!(f(&["%*d", "5", "42"]), None);
        assert_eq!(f(&["%99999d", "1"]), None);
        // A bare trailing `%` is an incomplete spec.
        assert_eq!(f(&["abc%", "x"]), None);
        // `%F` is not a valid Tcl conversion on any version — unlike
        // `%e`/`%E` or `%g`/`%G`, `%f` has no uppercase counterpart
        // (empirically verified: `tclsh8.6 format %F 3.14` → `bad field
        // specifier "F"`).
        assert_eq!(f(&["%F", "3.14"]), None);
        // A `ll`/`L` size modifier bails unconditionally: it is unmodelled
        // (its "no truncation" width isn't handled by any render_* helper),
        // and `%llu` specifically *raises* below Tcl 9.0 (empirically
        // verified: `tclsh8.6 format %llu 5` → "unsigned bignum format is
        // invalid"; 9.0 accepts it as `5`) while `%lld` skips the dialect's
        // normal 32-bit wrap on 9.0+ — neither is sound to fold as a plain
        // verb.
        assert_eq!(f(&["%llu", "5"]), None);
        assert_eq!(f(&["%lld", "5"]), None);
        assert_eq!(f(&["%Lf", "3.14"]), None);
    }

    #[test]
    fn format_folds_radix() {
        // Non-negative hex / octal with flags / width / precision — pinned
        // against tclsh8.6 + 9.0.
        assert_eq!(f(&["%x", "255"]).as_deref(), Some("ff"));
        assert_eq!(f(&["%X", "255"]).as_deref(), Some("FF"));
        assert_eq!(f(&["%o", "8"]).as_deref(), Some("10"));
        assert_eq!(f(&["%x", "0"]).as_deref(), Some("0"));
        assert_eq!(f(&["%08x", "255"]).as_deref(), Some("000000ff"));
        assert_eq!(f(&["%-8x", "255"]).as_deref(), Some("ff      "));
        assert_eq!(f(&["%5x", "255"]).as_deref(), Some("   ff"));
        assert_eq!(f(&["%.4x", "255"]).as_deref(), Some("00ff"));
        // `#` alternate form: sound only as the lowercase `0x` on a non-zero %x.
        assert_eq!(f(&["%#x", "255"]).as_deref(), Some("0xff"));
        assert_eq!(f(&["%#08x", "255"]).as_deref(), Some("0x0000ff"));
    }

    #[test]
    fn format_radix_flags_that_always_bail() {
        // `%#X` is `0XFF` (8.6) vs `0xFF` (9.0); `%#o` is `010` vs `0o10`;
        // `%#x 0` is `0x0` vs `0` — all bail on every version.
        assert_eq!(f(&["%#X", "255"]), None);
        assert_eq!(f(&["%#o", "8"]), None);
        assert_eq!(f(&["%#x", "0"]), None);
        // `+` / space don't apply to a radix conversion → bail.
        assert_eq!(f(&["%+x", "255"]), None);
    }

    #[test]
    fn format_unsigned_value_is_version_aware() {
        use TclVersion::{V8_6, V9_0};
        let u = |v: &str, ver| fold_format(&["%u", v], Some(ver));

        // Unsigned bit pattern at the dialect width: 9.0 32-bit, 8.x 64-bit.
        assert_eq!(u("-1", V9_0).as_deref(), Some("4294967295"));
        assert_eq!(u("-1", V8_6).as_deref(), Some("18446744073709551615"));
        assert_eq!(u("4294967296", V9_0).as_deref(), Some("0"));
        assert_eq!(u("4294967296", V8_6).as_deref(), Some("4294967296"));
        // Leading zero: octal on 8.x, decimal on 9.0.
        assert_eq!(u("010", V8_6).as_deref(), Some("8"));
        assert_eq!(u("010", V9_0).as_deref(), Some("10"));
        // Unversioned keeps the non-negative i32 subset (a negative bails).
        assert_eq!(fold_format(&["%u", "-1"], None), None);
        assert_eq!(fold_format(&["%u", "42"], None).as_deref(), Some("42"));
    }

    #[test]
    fn format_radix_value_is_version_aware() {
        use TclVersion::{V8_6, V9_0};
        let x = |v: &str, ver| fold_format(&["%x", v], Some(ver));

        // Negative: two's-complement at the dialect width (32-bit on 9.0,
        // 64-bit on 8.x) — verified vs all four tclsh.
        assert_eq!(x("-1", V9_0).as_deref(), Some("ffffffff"));
        assert_eq!(x("-1", V8_6).as_deref(), Some("ffffffffffffffff"));
        assert_eq!(x("-255", V9_0).as_deref(), Some("ffffff01"));
        assert_eq!(
            fold_format(&["%o", "-1"], Some(V9_0)).as_deref(),
            Some("37777777777")
        );
        assert_eq!(
            fold_format(&["%b", "-1"], Some(V9_0)).as_deref(),
            Some("11111111111111111111111111111111")
        );
        // Leading zero: octal on 8.x, decimal on 9.0 (`010` → 8 vs hex `a`).
        assert_eq!(x("010", V8_6).as_deref(), Some("8"));
        assert_eq!(x("010", V9_0).as_deref(), Some("a"));
        // Over i32: 9.0 wraps to 32-bit, 8.x keeps 64-bit.
        assert_eq!(x("4294967295", V9_0).as_deref(), Some("ffffffff"));
        assert_eq!(x("5000000000", V9_0).as_deref(), Some("2a05f200"));
        assert_eq!(x("5000000000", V8_6).as_deref(), Some("12a05f200"));
        // `%#x` prefix at the dialect width.
        assert_eq!(
            fold_format(&["%#x", "-1"], Some(V9_0)).as_deref(),
            Some("0xffffffff")
        );
        // Unversioned keeps the non-negative i32 subset (a negative bails).
        assert_eq!(fold_format(&["%x", "-1"], None), None);
        assert_eq!(fold_format(&["%x", "010"], None), None);
        assert_eq!(fold_format(&["%x", "255"], None).as_deref(), Some("ff"));
    }

    #[test]
    fn format_d_value_is_version_aware() {
        use TclVersion::{V8_4, V8_6, V9_0};
        let d = |v: &str, ver| fold_format(&["%d", v], Some(ver));

        // Leading zero: octal on 8.x, decimal on 9.0 (verified vs all four tclsh).
        assert_eq!(d("010", V8_6).as_deref(), Some("8"));
        assert_eq!(d("010", V9_0).as_deref(), Some("10"));
        assert_eq!(d("-010", V8_6).as_deref(), Some("-8"));
        assert_eq!(d("-010", V9_0).as_deref(), Some("-10"));
        // An invalid octal digit (8/9) raises on 8.x → bail; decimal on 9.0.
        assert_eq!(d("08", V8_6), None);
        assert_eq!(d("08", V9_0).as_deref(), Some("8"));
        // Out of i32: 9.0 wraps to signed-32-bit, 8.x keeps the full 64-bit value.
        assert_eq!(d("2147483648", V8_6).as_deref(), Some("2147483648"));
        assert_eq!(d("2147483648", V9_0).as_deref(), Some("-2147483648"));
        assert_eq!(d("5000000000", V8_6).as_deref(), Some("5000000000"));
        assert_eq!(d("5000000000", V9_0).as_deref(), Some("705032704"));
        assert_eq!(d("-2147483649", V9_0).as_deref(), Some("2147483647"));
        assert_eq!(d("4294967296", V9_0).as_deref(), Some("0"));
        // 8.x keeps i64 but bails beyond it (the >2^63 behaviour diverges, 8.5
        // mis-converts); 9.0 wraps any magnitude.
        assert_eq!(
            d("9223372036854775807", V8_6).as_deref(),
            Some("9223372036854775807")
        );
        assert_eq!(d("9223372036854775808", V8_6), None, "2^63 diverges on 8.x");
        // An unversioned dialect keeps the conservative i32 / no-leading-zero subset.
        assert_eq!(fold_format(&["%d", "010"], None), None);
        assert_eq!(fold_format(&["%d", "2147483648"], None), None);
        assert_eq!(d("8", V8_4).as_deref(), Some("8"), "plain decimal on 8.4");
    }

    #[test]
    fn format_binary_is_version_gated() {
        use TclVersion::{V8_4, V8_5, V8_6, V9_0};
        // `%b` raises before 8.6 → bail; folds (like `%x`) on 8.6+.
        assert_eq!(fold_format(&["%b", "5"], Some(V8_4)), None);
        assert_eq!(fold_format(&["%b", "5"], Some(V8_5)), None);
        assert_eq!(fold_format(&["%b", "5"], None), None, "unknown version");
        assert_eq!(
            fold_format(&["%b", "5"], Some(V8_6)).as_deref(),
            Some("101")
        );
        assert_eq!(
            fold_format(&["%b", "255"], Some(V9_0)).as_deref(),
            Some("11111111")
        );
        assert_eq!(
            fold_format(&["%08b", "5"], Some(V9_0)).as_deref(),
            Some("00000101")
        );
        assert_eq!(
            fold_format(&["%#b", "5"], Some(V9_0)).as_deref(),
            Some("0b101")
        );
        // `%#b 0` diverges (0b0 on 8.6 vs 0 on 9.0) → bail; a negative folds to
        // the dialect-width two's complement (covered by
        // `format_radix_value_is_version_aware`).
        assert_eq!(fold_format(&["%#b", "0"], Some(V9_0)), None);
    }

    #[test]
    fn format_folds_char() {
        // Printable-ASCII codepoints fold; width / `-` justify apply.
        assert_eq!(f(&["%c", "65"]).as_deref(), Some("A"));
        assert_eq!(f(&["%c", "97"]).as_deref(), Some("a"));
        assert_eq!(f(&["%c", "126"]).as_deref(), Some("~"));
        assert_eq!(f(&["%5c", "65"]).as_deref(), Some("    A"));
        assert_eq!(f(&["%-5c", "65"]).as_deref(), Some("A    "));
        // Non-printable / out-of-range / divergent codepoints bail: NUL and
        // control chars (< 32), DEL (127), codepoint >= 128 (8.6 emits a byte,
        // 9.0 raises), and negatives.
        assert_eq!(f(&["%c", "0"]), None);
        assert_eq!(f(&["%c", "10"]), None);
        assert_eq!(f(&["%c", "127"]), None);
        assert_eq!(f(&["%c", "256"]), None);
        assert_eq!(f(&["%c", "-1"]), None);
        // Numeric flags / precision don't apply to `%c` → bail.
        assert_eq!(f(&["%#c", "65"]), None);
        assert_eq!(f(&["%.2c", "65"]), None);
    }

    #[test]
    fn format_folds_fixed_float() {
        // Default precision is 6; rounding is round-half-to-even (matches C/Tcl).
        assert_eq!(f(&["%f", "3.14"]).as_deref(), Some("3.140000"));
        assert_eq!(f(&["%.2f", "3.14159"]).as_deref(), Some("3.14"));
        assert_eq!(f(&["%.0f", "2.5"]).as_deref(), Some("2"));
        assert_eq!(f(&["%.0f", "3.5"]).as_deref(), Some("4"));
        assert_eq!(f(&["%.2f", "0.125"]).as_deref(), Some("0.12"));
        assert_eq!(f(&["%f", "42"]).as_deref(), Some("42.000000"));
        // Integer args fold; the value is parsed as a double.
        assert_eq!(f(&["%.2f", "-2.5"]).as_deref(), Some("-2.50"));
        // Flags / width / precision — the `0` flag zero-pads even with a
        // precision (unlike `%d`).
        assert_eq!(f(&["%8.2f", "3.14159"]).as_deref(), Some("    3.14"));
        assert_eq!(f(&["%-8.2f", "3.14159"]).as_deref(), Some("3.14    "));
        assert_eq!(f(&["%08.3f", "2.5"]).as_deref(), Some("0002.500"));
        assert_eq!(f(&["%+.2f", "3.14"]).as_deref(), Some("+3.14"));
        assert_eq!(f(&["% .2f", "3.14"]).as_deref(), Some(" 3.14"));
        // Non-finite, the `#` alternate form, and non-double args bail.
        assert_eq!(f(&["%f", "inf"]), None);
        assert_eq!(f(&["%f", "nan"]), None);
        assert_eq!(f(&["%#f", "3.14"]), None);
        assert_eq!(f(&["%f", "abc"]), None);
        assert_eq!(f(&["%f", "0x1f"]), None);
        assert_eq!(f(&["%f", "1_000"]), None);
    }

    #[test]
    fn format_folds_scientific_and_general() {
        // %e / %E: C-shaped exponent (sign + ≥2 digits), default precision 6.
        assert_eq!(f(&["%e", "31400.0"]).as_deref(), Some("3.140000e+04"));
        assert_eq!(f(&["%.3e", "31400.0"]).as_deref(), Some("3.140e+04"));
        assert_eq!(f(&["%E", "31400.0"]).as_deref(), Some("3.140000E+04"));
        assert_eq!(f(&["%e", "0.0314"]).as_deref(), Some("3.140000e-02"));
        assert_eq!(f(&["%.2e", "9.999"]).as_deref(), Some("1.00e+01"));
        assert_eq!(f(&["%e", "1e100"]).as_deref(), Some("1.000000e+100"));
        // %g / %G: shorter of %e/%f, trailing zeros stripped.
        assert_eq!(f(&["%g", "3.14"]).as_deref(), Some("3.14"));
        assert_eq!(f(&["%g", "100000.0"]).as_deref(), Some("100000"));
        assert_eq!(f(&["%g", "1000000.0"]).as_deref(), Some("1e+06"));
        assert_eq!(f(&["%g", "0.0001"]).as_deref(), Some("0.0001"));
        assert_eq!(f(&["%g", "0.00001"]).as_deref(), Some("1e-05"));
        assert_eq!(f(&["%g", "1.0"]).as_deref(), Some("1"));
        assert_eq!(f(&["%.3g", "0.000123456"]).as_deref(), Some("0.000123"));
        assert_eq!(f(&["%G", "1234567.0"]).as_deref(), Some("1.23457E+06"));
        // Flags / width apply (the `0` flag zero-pads even with a precision).
        assert_eq!(f(&["%+.2e", "31400.0"]).as_deref(), Some("+3.14e+04"));
        assert_eq!(
            f(&["%015.3e", "31400.0"]).as_deref(),
            Some("0000003.140e+04")
        );
    }
}
