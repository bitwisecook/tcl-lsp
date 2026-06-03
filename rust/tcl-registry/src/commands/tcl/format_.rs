//! `format` — format a string using printf-style conversion.
use crate::prelude::*;

/// SYNC-JUN02d (#525 B-tail) + SYNC-JUN03 follow-up: constant-fold the
/// integer (`%d` / `%i` / `%u`), radix (`%x` / `%X` / `%o`), character
/// (`%c`), float (`%f` / `%F` / `%e` / `%E` / `%g` / `%G`) and string (`%s`)
/// conversions of `format`, with the printf flag / width / precision matrix
/// that is **byte-identical across Tcl 8.4 → 9.0** (verified differentially
/// against `tclsh8.4`/`8.5`/`8.6`/`9.0` — see `tests/differential_fold.rs`).
/// Only `%b` (raises before 8.6), size modifiers, and `*`/positional specs
/// remain unfolded — `%b` is permanently unsound (a fold would turn an old-
/// dialect error into a value).
///
/// `args[0]` is the format string (ASCII-restricted — we byte-index it for
/// the `%` scan, so a multi-byte char would be mis-sliced); `args[1..]` are
/// the values.  Soundness first — fold only the dialect-invariant subset and
/// bail (return `None`, leaving the call unfolded) on everything else, since a
/// wrong fold is a false-positive optimisation.  Each conversion's exact
/// constraints live on its `render_*` helper; in summary:
///
/// * `%d` / `%i` / `%x` / `%X` / `%o` / `%c` accept only a plain decimal
///   argument the whole 8.4 → 9.0 range agrees on — see [`parse_decimal_arg`]
///   ([`Conversion::render_radix`] additionally requires non-negative,
///   [`Conversion::render_char`] a printable-ASCII codepoint).  The `#`
///   alternate form is sound only as the lowercase `0x` on a non-zero `%x`.
/// * `%s` honours `-` / width / precision; the numeric flags bail (Tcl
///   ignores them on strings), and width/precision over a non-ASCII value
///   bails (the character count diverges 8.x UTF-16 units ↔ 9.0 codepoints).
/// * the float conversions parse a double over the same subset as `string is
///   double` ([`parse_float_arg`]) and build on Rust's float formatter, with
///   `%e`/`%g`'s exponent / shortest-form rendering reshaped to C's by
///   [`fmt_sci`] / [`fmt_general`]; a non-finite value and the `#` form bail.
///   See [`Conversion::render_float`].
/// * `%b` (raises before 8.6), size modifiers (`%ld`), `*` (arg-driven)
///   width / precision, positional `%n$`, and any field over [`MAX_FIELD`]
///   all bail.
/// * A bare trailing `%` (an incomplete conversion, which Tcl raises on) and
///   too few arguments both bail; extra arguments are ignored (matching Tcl).
fn fold_format(args: &[&str]) -> Option<String> {
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
        let value = vals.get(ai)?;
        ai += 1;
        out.push_str(&conv.render(value)?);
    }
    Some(out)
}

/// Field sizes beyond this bail — we never fold a literal into kilobytes of
/// padding (sound: a missed fold is never wrong).
const MAX_FIELD: usize = 1000;

bitflags::bitflags! {
    /// The printf conversion flags parsed from a `%…` spec.
    #[derive(Clone, Copy)]
    struct FmtFlags: u8 {
        const MINUS = 1 << 0; // `-`  left-justify
        const PLUS = 1 << 1; // `+`  always show a sign
        const SPACE = 1 << 2; // ` `  space before a non-negative number
        const ZERO = 1 << 3; // `0`  zero-pad
        const HASH = 1 << 4; // `#`  alternate form
    }
}

/// A single parsed `%…` conversion (the modelled subset).
struct Conversion {
    flags: FmtFlags,
    width: Option<usize>,
    precision: Option<usize>,
    verb: u8,
}

/// The outcome of parsing a width / `.precision` field.
enum Field {
    /// No digits were present.
    Absent,
    /// A parsed field size.
    Size(usize),
}

/// The base / case of a radix conversion (`%x` / `%X` / `%o`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Radix {
    HexLower,
    HexUpper,
    Octal,
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
    /// Parse flags / width / `.precision` / verb, starting just past the `%`
    /// and advancing `i` past the verb.  Bails on `*` width / precision, an
    /// over-[`MAX_FIELD`] field, or a missing verb.
    fn parse(fmt: &[u8], i: &mut usize) -> Option<Self> {
        let mut flags = FmtFlags::empty();
        loop {
            let bit = match fmt.get(*i) {
                Some(b'-') => FmtFlags::MINUS,
                Some(b'+') => FmtFlags::PLUS,
                Some(b' ') => FmtFlags::SPACE,
                Some(b'0') => FmtFlags::ZERO,
                Some(b'#') => FmtFlags::HASH,
                _ => break,
            };
            flags |= bit;
            *i += 1;
        }
        if fmt.get(*i) == Some(&b'*') {
            return None; // arg-driven width — unmodelled
        }
        let width = match parse_field(fmt, i)? {
            Field::Absent => None,
            Field::Size(n) => Some(n),
        };
        let precision = if fmt.get(*i) == Some(&b'.') {
            *i += 1;
            if fmt.get(*i) == Some(&b'*') {
                return None; // arg-driven precision — unmodelled
            }
            // a `.` with no digits means precision 0
            Some(match parse_field(fmt, i)? {
                Field::Absent => 0,
                Field::Size(n) => n,
            })
        } else {
            None
        };
        let verb = *fmt.get(*i)?;
        *i += 1;
        Some(Self {
            flags,
            width,
            precision,
            verb,
        })
    }

    /// Render this conversion for `value`, or bail (`None`) for an unmodelled
    /// verb or an out-of-subset argument.
    fn render(&self, value: &str) -> Option<String> {
        match self.verb {
            b'd' | b'i' => self.render_int(value),
            b'x' => self.render_radix(value, Radix::HexLower),
            b'X' => self.render_radix(value, Radix::HexUpper),
            b'o' => self.render_radix(value, Radix::Octal),
            b'c' => self.render_char(value),
            b'f' | b'F' => self.render_float(value, FloatKind::Fixed, false),
            b'e' => self.render_float(value, FloatKind::Scientific, false),
            b'E' => self.render_float(value, FloatKind::Scientific, true),
            b'g' => self.render_float(value, FloatKind::General, false),
            b'G' => self.render_float(value, FloatKind::General, true),
            b'u' => self.render_unsigned(value),
            b's' => self.render_str(value),
            // `%b` (raises before 8.6), size modifiers (`%ld`) and positional
            // `%n$` bail — `%b` is permanently unsound (a fold would turn an
            // 8.4/8.5 error into a value).
            _ => None,
        }
    }

    /// Render `%u` (unsigned decimal).  The argument must be a **plain
    /// non-negative** decimal in the unsigned-32-bit range every release
    /// shares (`[0, 2³²-1]`): a negative value (`%u -1` → `…615` on 8.x vs
    /// `4294967295` on 9.0), a value past `2³²` (`%u 4294967296` → `0` on 9.0),
    /// a sign, a leading zero (octal in 8.x), or a `0x`/etc. form all diverge
    /// or differ and bail.  `%u` carries no sign, so the `+`/space flags don't
    /// apply (Tcl ignores them); `#` bails.
    fn render_unsigned(&self, value: &str) -> Option<String> {
        if self.flags.contains(FmtFlags::HASH) {
            return None;
        }
        let t = value.trim();
        let b = t.as_bytes();
        if b.is_empty() || !b.iter().all(u8::is_ascii_digit) {
            return None; // a sign / non-digit / empty bails
        }
        if b.len() > 1 && b[0] == b'0' {
            return None; // leading zero → octal in 8.x
        }
        let n: u64 = t.parse().ok()?;
        if n > 4_294_967_295 {
            return None; // beyond the unsigned-32-bit range all versions share
        }
        let mut digits = n.to_string();
        if let Some(p) = self.precision {
            if digits.len() < p {
                digits = "0".repeat(p - digits.len()) + &digits;
            }
        }
        Some(self.pad("", &digits, self.int_zero_pad()))
    }

    /// Render a float conversion — `%f`/`%F` (fixed), `%e`/`%E` (scientific),
    /// or `%g`/`%G` (general).  All build on Rust's float formatter, which is
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

    fn render_int(&self, value: &str) -> Option<String> {
        if self.flags.contains(FmtFlags::HASH) {
            return None; // `%#d` diverges 8.6 (`42`) ↔ 9.0 (`0d42`)
        }
        let n = parse_decimal_arg(value)?;
        let mut digits = n.unsigned_abs().to_string();
        if let Some(p) = self.precision {
            if digits.len() < p {
                digits = "0".repeat(p - digits.len()) + &digits;
            }
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

    /// Render `%x` / `%X` / `%o` for a **non-negative** dialect-invariant
    /// argument.  Negative values bail (the two's-complement digit count is
    /// 32-bit in 9.0 but 64-bit in 8.6).  `+` / space don't apply to a radix
    /// conversion (bail).  The `#` alternate form is sound only as the
    /// lowercase `0x` prefix on a non-zero `%x`: `%#X` is `0XFF` on 8.6 but
    /// `0xFF` on 9.0, `%#o` is `010` vs `0o10`, and `%#x 0` is `0x0` vs `0`.
    fn render_radix(&self, value: &str, radix: Radix) -> Option<String> {
        if self.flags.intersects(FmtFlags::PLUS | FmtFlags::SPACE) {
            return None;
        }
        // A negative value bails: its two's-complement digit count is 32-bit
        // in 9.0 but 64-bit in 8.6 (`try_from` rejects the sign for us).
        let n = u64::try_from(parse_decimal_arg(value)?).ok()?;
        let mut digits = match radix {
            Radix::HexLower => format!("{n:x}"),
            Radix::HexUpper => format!("{n:X}"),
            Radix::Octal => format!("{n:o}"),
        };
        let prefix = if self.flags.contains(FmtFlags::HASH) {
            if radix == Radix::HexLower && n != 0 {
                "0x"
            } else {
                return None; // `#X` / `#o` / `#x 0` diverge across versions
            }
        } else {
            ""
        };
        if let Some(p) = self.precision {
            if digits.len() < p {
                digits = "0".repeat(p - digits.len()) + &digits;
            }
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

/// Parse an optional run of ASCII decimal digits as a field size, capped at
/// [`MAX_FIELD`].  Returns [`Field::Absent`] when no digit is present,
/// [`Field::Size`] for a value, or `None` (bail) on overflow or over-cap.
fn parse_field(fmt: &[u8], i: &mut usize) -> Option<Field> {
    let start = *i;
    let mut n = 0usize;
    while let Some(&d) = fmt.get(*i) {
        if !d.is_ascii_digit() {
            break;
        }
        n = n.checked_mul(10)?.checked_add((d - b'0') as usize)?;
        if n > MAX_FIELD {
            return None;
        }
        *i += 1;
    }
    Some(if *i == start {
        Field::Absent
    } else {
        Field::Size(n)
    })
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
        traits: Traits::BYTE_COMPILED | Traits::PURE | Traits::CSE_CANDIDATE,
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        const_fold: Some(fold_format),
        hover: Some(HoverSnippet::brief(
            "Format a string.",
            &["format formatString ?arg ...?"],
            "Tcl format(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::fold_format;

    #[test]
    fn format_folds_plain_s_d_percent_subset() {
        assert_eq!(fold_format(&["hello"]).as_deref(), Some("hello"));
        assert_eq!(fold_format(&["%s", "world"]).as_deref(), Some("world"));
        assert_eq!(fold_format(&["%d", "42"]).as_deref(), Some("42"));
        assert_eq!(fold_format(&["%d", " -7 "]).as_deref(), Some("-7"));
        assert_eq!(
            fold_format(&["v=%s n=%d", "x", "3"]).as_deref(),
            Some("v=x n=3")
        );
        assert_eq!(fold_format(&["100%%"]).as_deref(), Some("100%"));
        // `%d` with a non-integer arg bails (Tcl errors at runtime).
        assert_eq!(fold_format(&["%d", "hi"]), None);
        // Too few args bails.
        assert_eq!(fold_format(&["%s %s", "only"]), None);
    }

    #[test]
    fn format_folds_integer_flag_width_precision() {
        // width / justify / zero-pad / sign — pinned against tclsh8.6 + 9.0.
        assert_eq!(fold_format(&["%5d", "42"]).as_deref(), Some("   42"));
        assert_eq!(fold_format(&["%-5d", "42"]).as_deref(), Some("42   "));
        assert_eq!(fold_format(&["%05d", "7"]).as_deref(), Some("00007"));
        assert_eq!(fold_format(&["%05d", "-7"]).as_deref(), Some("-0007"));
        assert_eq!(fold_format(&["%+d", "42"]).as_deref(), Some("+42"));
        assert_eq!(fold_format(&["% d", "42"]).as_deref(), Some(" 42"));
        assert_eq!(fold_format(&["% d", "-7"]).as_deref(), Some("-7"));
        assert_eq!(fold_format(&["%+05d", "42"]).as_deref(), Some("+0042"));
        assert_eq!(fold_format(&["%5d", "-42"]).as_deref(), Some("  -42"));
        assert_eq!(fold_format(&["%i", "42"]).as_deref(), Some("42")); // %i alias
                                                                       // precision = minimum digits; the `0` flag is suppressed by precision.
        assert_eq!(fold_format(&["%.5d", "42"]).as_deref(), Some("00042"));
        assert_eq!(fold_format(&["%.3d", "-4"]).as_deref(), Some("-004"));
        assert_eq!(fold_format(&["%5.3d", "42"]).as_deref(), Some("  042"));
        assert_eq!(fold_format(&["%05.3d", "42"]).as_deref(), Some("  042"));
        assert_eq!(fold_format(&["%.0d", "0"]).as_deref(), Some("0"));
    }

    #[test]
    fn format_folds_string_width_precision() {
        assert_eq!(fold_format(&["%10s", "hi"]).as_deref(), Some("        hi"));
        assert_eq!(fold_format(&["%-10s", "hi"]).as_deref(), Some("hi        "));
        assert_eq!(fold_format(&["%.3s", "hello"]).as_deref(), Some("hel"));
        assert_eq!(fold_format(&["%5.3s", "hello"]).as_deref(), Some("  hel"));
        assert_eq!(fold_format(&["%.0s", "hi"]).as_deref(), Some(""));
    }

    #[test]
    fn format_bails_on_version_divergent_and_unmodelled() {
        // `%#d` is `0d42` on Tcl 9 but `42` on 8.6 — no sound fold.
        assert_eq!(fold_format(&["%#d", "42"]), None);
        // Leading zero is octal in 8.x but decimal in 9.0 (`%d 010` → 8 vs 10).
        assert_eq!(fold_format(&["%d", "010"]), None);
        assert_eq!(fold_format(&["%05d", "010"]), None);
        // Past 2^31 `%d` wraps to 32 bits in 9.0 but not 8.6.
        assert_eq!(fold_format(&["%d", "2147483648"]), None);
        assert_eq!(fold_format(&["%d", "-2147483649"]), None);
        // …but the 32-bit boundary itself folds.
        assert_eq!(
            fold_format(&["%d", "2147483647"]).as_deref(),
            Some("2147483647")
        );
        assert_eq!(
            fold_format(&["%d", "-2147483648"]).as_deref(),
            Some("-2147483648")
        );
        // Hex / octal / binary prefixes bail (Rust's parser declines them).
        assert_eq!(fold_format(&["%d", "0x10"]), None);
        // %u folds for a non-negative in-range value; divergent forms bail.
        assert_eq!(fold_format(&["%u", "42"]).as_deref(), Some("42"));
        assert_eq!(fold_format(&["%05u", "42"]).as_deref(), Some("00042"));
        assert_eq!(fold_format(&["%.3u", "42"]).as_deref(), Some("042"));
        assert_eq!(
            fold_format(&["%u", "2147483648"]).as_deref(),
            Some("2147483648")
        );
        assert_eq!(fold_format(&["%u", "-1"]), None); // unsigned-wrap diverges
        assert_eq!(fold_format(&["%u", "4294967296"]), None); // past 2^32
        assert_eq!(fold_format(&["%u", "010"]), None); // leading zero (octal in 8.x)
                                                       // %b raises before Tcl 8.6 → permanently unsound to fold.
        assert_eq!(fold_format(&["%b", "5"]), None);
        // Float verbs with a non-finite value bail (Inf/NaN spelling is edgy);
        // the `#` alternate form bails (it changes %g trailing-zero stripping).
        assert_eq!(fold_format(&["%f", "inf"]), None);
        assert_eq!(fold_format(&["%e", "nan"]), None);
        assert_eq!(fold_format(&["%#f", "3.14"]), None);
        assert_eq!(fold_format(&["%#g", "3.14"]), None);
        // Numeric flags don't apply to `%s` → bail.
        assert_eq!(fold_format(&["%05s", "hi"]), None);
        assert_eq!(fold_format(&["%+s", "hi"]), None);
        // Non-ASCII value under a width/precision bails (char-count diverges).
        assert_eq!(fold_format(&["%5s", "café"]), None);
        // Arg-driven width / precision and over-cap fields bail.
        assert_eq!(fold_format(&["%*d", "5", "42"]), None);
        assert_eq!(fold_format(&["%99999d", "1"]), None);
        // A bare trailing `%` is an incomplete spec.
        assert_eq!(fold_format(&["abc%", "x"]), None);
    }

    #[test]
    fn format_folds_radix() {
        // Non-negative hex / octal with flags / width / precision — pinned
        // against tclsh8.6 + 9.0.
        assert_eq!(fold_format(&["%x", "255"]).as_deref(), Some("ff"));
        assert_eq!(fold_format(&["%X", "255"]).as_deref(), Some("FF"));
        assert_eq!(fold_format(&["%o", "8"]).as_deref(), Some("10"));
        assert_eq!(fold_format(&["%x", "0"]).as_deref(), Some("0"));
        assert_eq!(fold_format(&["%08x", "255"]).as_deref(), Some("000000ff"));
        assert_eq!(fold_format(&["%-8x", "255"]).as_deref(), Some("ff      "));
        assert_eq!(fold_format(&["%5x", "255"]).as_deref(), Some("   ff"));
        assert_eq!(fold_format(&["%.4x", "255"]).as_deref(), Some("00ff"));
        // `#` alternate form: sound only as the lowercase `0x` on a non-zero %x.
        assert_eq!(fold_format(&["%#x", "255"]).as_deref(), Some("0xff"));
        assert_eq!(fold_format(&["%#08x", "255"]).as_deref(), Some("0x0000ff"));
    }

    #[test]
    fn format_bails_on_version_divergent_radix() {
        // Negative → two's-complement digit count is 32-bit (9.0) vs 64-bit (8.6).
        assert_eq!(fold_format(&["%x", "-1"]), None);
        assert_eq!(fold_format(&["%o", "-1"]), None);
        // `%#X` is `0XFF` (8.6) vs `0xFF` (9.0); `%#o` is `010` vs `0o10`;
        // `%#x 0` is `0x0` vs `0` — all bail.
        assert_eq!(fold_format(&["%#X", "255"]), None);
        assert_eq!(fold_format(&["%#o", "8"]), None);
        assert_eq!(fold_format(&["%#x", "0"]), None);
        // `+` / space don't apply to a radix conversion → bail.
        assert_eq!(fold_format(&["%+x", "255"]), None);
        // Leading-zero / over-range args bail just as for `%d`.
        assert_eq!(fold_format(&["%x", "010"]), None);
        assert_eq!(fold_format(&["%x", "4294967295"]), None);
    }

    #[test]
    fn format_folds_char() {
        // Printable-ASCII codepoints fold; width / `-` justify apply.
        assert_eq!(fold_format(&["%c", "65"]).as_deref(), Some("A"));
        assert_eq!(fold_format(&["%c", "97"]).as_deref(), Some("a"));
        assert_eq!(fold_format(&["%c", "126"]).as_deref(), Some("~"));
        assert_eq!(fold_format(&["%5c", "65"]).as_deref(), Some("    A"));
        assert_eq!(fold_format(&["%-5c", "65"]).as_deref(), Some("A    "));
        // Non-printable / out-of-range / divergent codepoints bail: NUL and
        // control chars (< 32), DEL (127), codepoint >= 128 (8.6 emits a byte,
        // 9.0 raises), and negatives.
        assert_eq!(fold_format(&["%c", "0"]), None);
        assert_eq!(fold_format(&["%c", "10"]), None);
        assert_eq!(fold_format(&["%c", "127"]), None);
        assert_eq!(fold_format(&["%c", "256"]), None);
        assert_eq!(fold_format(&["%c", "-1"]), None);
        // Numeric flags / precision don't apply to `%c` → bail.
        assert_eq!(fold_format(&["%#c", "65"]), None);
        assert_eq!(fold_format(&["%.2c", "65"]), None);
    }

    #[test]
    fn format_folds_fixed_float() {
        // Default precision is 6; rounding is round-half-to-even (matches C/Tcl).
        assert_eq!(fold_format(&["%f", "3.14"]).as_deref(), Some("3.140000"));
        assert_eq!(fold_format(&["%.2f", "3.14159"]).as_deref(), Some("3.14"));
        assert_eq!(fold_format(&["%.0f", "2.5"]).as_deref(), Some("2"));
        assert_eq!(fold_format(&["%.0f", "3.5"]).as_deref(), Some("4"));
        assert_eq!(fold_format(&["%.2f", "0.125"]).as_deref(), Some("0.12"));
        assert_eq!(fold_format(&["%f", "42"]).as_deref(), Some("42.000000"));
        // Integer args fold; the value is parsed as a double.
        assert_eq!(fold_format(&["%.2f", "-2.5"]).as_deref(), Some("-2.50"));
        // Flags / width / precision — the `0` flag zero-pads even with a
        // precision (unlike `%d`).
        assert_eq!(
            fold_format(&["%8.2f", "3.14159"]).as_deref(),
            Some("    3.14")
        );
        assert_eq!(
            fold_format(&["%-8.2f", "3.14159"]).as_deref(),
            Some("3.14    ")
        );
        assert_eq!(fold_format(&["%08.3f", "2.5"]).as_deref(), Some("0002.500"));
        assert_eq!(fold_format(&["%+.2f", "3.14"]).as_deref(), Some("+3.14"));
        assert_eq!(fold_format(&["% .2f", "3.14"]).as_deref(), Some(" 3.14"));
        // Non-finite, the `#` alternate form, and non-double args bail.
        assert_eq!(fold_format(&["%f", "inf"]), None);
        assert_eq!(fold_format(&["%f", "nan"]), None);
        assert_eq!(fold_format(&["%#f", "3.14"]), None);
        assert_eq!(fold_format(&["%f", "abc"]), None);
        assert_eq!(fold_format(&["%f", "0x1f"]), None);
        assert_eq!(fold_format(&["%f", "1_000"]), None);
    }

    #[test]
    fn format_folds_scientific_and_general() {
        // %e / %E: C-shaped exponent (sign + ≥2 digits), default precision 6.
        assert_eq!(
            fold_format(&["%e", "31400.0"]).as_deref(),
            Some("3.140000e+04")
        );
        assert_eq!(
            fold_format(&["%.3e", "31400.0"]).as_deref(),
            Some("3.140e+04")
        );
        assert_eq!(
            fold_format(&["%E", "31400.0"]).as_deref(),
            Some("3.140000E+04")
        );
        assert_eq!(
            fold_format(&["%e", "0.0314"]).as_deref(),
            Some("3.140000e-02")
        );
        assert_eq!(fold_format(&["%.2e", "9.999"]).as_deref(), Some("1.00e+01"));
        assert_eq!(
            fold_format(&["%e", "1e100"]).as_deref(),
            Some("1.000000e+100")
        );
        // %g / %G: shorter of %e/%f, trailing zeros stripped.
        assert_eq!(fold_format(&["%g", "3.14"]).as_deref(), Some("3.14"));
        assert_eq!(fold_format(&["%g", "100000.0"]).as_deref(), Some("100000"));
        assert_eq!(fold_format(&["%g", "1000000.0"]).as_deref(), Some("1e+06"));
        assert_eq!(fold_format(&["%g", "0.0001"]).as_deref(), Some("0.0001"));
        assert_eq!(fold_format(&["%g", "0.00001"]).as_deref(), Some("1e-05"));
        assert_eq!(fold_format(&["%g", "1.0"]).as_deref(), Some("1"));
        assert_eq!(
            fold_format(&["%.3g", "0.000123456"]).as_deref(),
            Some("0.000123")
        );
        assert_eq!(
            fold_format(&["%G", "1234567.0"]).as_deref(),
            Some("1.23457E+06")
        );
        // Flags / width apply (the `0` flag zero-pads even with a precision).
        assert_eq!(
            fold_format(&["%+.2e", "31400.0"]).as_deref(),
            Some("+3.14e+04")
        );
        assert_eq!(
            fold_format(&["%015.3e", "31400.0"]).as_deref(),
            Some("0000003.140e+04")
        );
    }
}
