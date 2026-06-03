//! `format` — format a string using printf-style conversion.
use crate::prelude::*;

/// SYNC-JUN02d (#525 B-tail) + SYNC-JUN03 follow-up: constant-fold the
/// decimal-integer (`%d` / `%i`) and string (`%s`) conversions of `format`,
/// with the full flag / width / precision matrix that is **byte-identical
/// across Tcl 8.4 → 9.0** (verified differentially against `tclsh8.6` +
/// `tclsh9.0` — see `tests/differential_fold.rs`).  This extends the original
/// plain `%s` / `%d` / `%%` subset with the printf flags `-` (left-justify),
/// `+` (always sign), space (sign placeholder) and `0` (zero-pad), an
/// optional width, and an optional `.precision` (minimum digits for integers,
/// maximum characters for strings).
///
/// `args[0]` is the format string (ASCII-restricted — we byte-index it for
/// the `%` scan, so a multi-byte char would be mis-sliced); `args[1..]` are
/// the values.  Soundness first — fold only the dialect-invariant subset and
/// bail (return `None`, leaving the call unfolded) on everything else, since a
/// wrong fold is a false-positive optimisation:
///
/// * `%d` / `%i` accept only a plain decimal argument the whole 8.4 → 9.0
///   range agrees on — see [`parse_decimal_arg`].  The `#` flag bails (Tcl 9
///   renders `%#d 42` as `0d42`, Tcl 8.6 as `42`).
/// * `%s` honours `-`, width and precision; the numeric flags (`+`, space,
///   `0`, `#`) bail (Tcl ignores them on strings, so bailing is never wrong),
///   and a width/precision on a non-ASCII value bails (the character count
///   diverges across Tcl 8.x UTF-16 units and 9.0 codepoints).
/// * The radix conversions (`%x` / `%X` / `%o` / `%b`), float conversions
///   (`%f` / `%e` / `%g`), `%c`, `%u`, size modifiers (`%ld`), `*` (arg-driven)
///   width / precision, positional `%n$`, and any field over [`MAX_FIELD`] all
///   bail — radix and float folds are differentially pinned in later strips.
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
            b's' => self.render_str(value),
            // Float (`%f`/`%e`/`%g`), `%c`, `%u`, `%b`, size modifiers (`%ld`)
            // and positional `%n$` are differentially pinned in later strips —
            // bail for now.
            _ => None,
        }
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
        Some(self.pad(sign, &digits))
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
        Some(self.pad(prefix, &digits))
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
    /// `-`/`+`/space for `%d`, or a `0x` radix marker for `%#x`): zero-pad
    /// between the prefix and digits (only with the `0` flag and no precision —
    /// precision suppresses zero-padding, per C/Tcl), left-justify with spaces
    /// under `-`, else right-justify with spaces.
    fn pad(&self, prefix: &str, digits: &str) -> String {
        let width = self.width.unwrap_or(0);
        let body_len = prefix.len() + digits.len();
        if body_len >= width {
            return format!("{prefix}{digits}");
        }
        let fill = width - body_len;
        if self.flags.contains(FmtFlags::MINUS) {
            format!("{prefix}{digits}{}", " ".repeat(fill))
        } else if self.flags.contains(FmtFlags::ZERO) && self.precision.is_none() {
            format!("{prefix}{}{digits}", "0".repeat(fill))
        } else {
            format!("{}{prefix}{digits}", " ".repeat(fill))
        }
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
        // Float / %c / %b verbs are deferred to a later strip.
        assert_eq!(fold_format(&["%5.2f", "3.14159"]), None);
        assert_eq!(fold_format(&["%c", "65"]), None);
        assert_eq!(fold_format(&["%b", "5"]), None);
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
}
