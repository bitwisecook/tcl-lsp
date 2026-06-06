//! Tcl `format` conversion-specifier grammar — the shared spec parser.
//!
//! Parses one `%…` conversion (`%[flags][width][.precision]verb`) into a
//! [`Spec`], the structured form both consumers render from: the LSP/compiler's
//! version-aware const-folder (`tcl-registry`) and the runtime port's renderer
//! over its own value type. Rendering is **not** here — it is value-type- and
//! dialect-specific, so each consumer owns it; this module is the one place the
//! specifier *grammar* lives (reference Tcl 9.0 `Tcl_AppendFormatToObj`,
//! `tmp/tcl9.0.3/generic/tclStringObj.c`).
//!
//! The modelled subset bails (`None`) on arg-driven `*` width/precision, an
//! over-[`MAX_FIELD`] field, positional `%n$`, and size modifiers — a missed
//! parse is never wrong for a const-fold, and the runtime can extend it.

/// Field sizes beyond this bail — never fold a literal into kilobytes of
/// padding (sound: a missed fold is never wrong).
pub const MAX_FIELD: usize = 1000;

bitflags::bitflags! {
    /// The printf conversion flags parsed from a `%…` spec.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FmtFlags: u8 {
        /// `-` left-justify.
        const MINUS = 1 << 0;
        /// `+` always show a sign.
        const PLUS = 1 << 1;
        /// ` ` space before a non-negative number.
        const SPACE = 1 << 2;
        /// `0` zero-pad.
        const ZERO = 1 << 3;
        /// `#` alternate form.
        const HASH = 1 << 4;
    }
}

/// A single parsed `%…` conversion (the modelled subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    /// The parsed flag set.
    pub flags: FmtFlags,
    /// Field width, if present.
    pub width: Option<usize>,
    /// `.precision`, if present (a bare `.` means `0`).
    pub precision: Option<usize>,
    /// The conversion verb byte (`d`/`s`/`x`/…).
    pub verb: u8,
}

/// The outcome of parsing a width / `.precision` field.
enum Field {
    /// No digits were present.
    Absent,
    /// A parsed field size.
    Size(usize),
}

/// Parse one conversion's flags / width / `.precision` / verb, starting just
/// past the `%` and advancing `i` past the verb. Bails on `*` width / precision,
/// an over-[`MAX_FIELD`] field, or a missing verb.
pub fn parse_spec(fmt: &[u8], i: &mut usize) -> Option<Spec> {
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
    Some(Spec {
        flags,
        width,
        precision,
        verb,
    })
}

/// Parse a run of decimal digits as a width / precision field, advancing `i`.
/// Bails (`None`) when the value exceeds [`MAX_FIELD`] or overflows.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Option<(Spec, usize)> {
        let mut i = 0;
        let spec = parse_spec(s.as_bytes(), &mut i)?;
        Some((spec, i))
    }

    #[test]
    fn plain_verb() {
        let (s, i) = parse("d").unwrap();
        assert_eq!(s.flags, FmtFlags::empty());
        assert_eq!(s.width, None);
        assert_eq!(s.precision, None);
        assert_eq!(s.verb, b'd');
        assert_eq!(i, 1);
    }

    #[test]
    fn flags_width_precision() {
        let (s, i) = parse("-+08.3f").unwrap();
        assert!(s
            .flags
            .contains(FmtFlags::MINUS | FmtFlags::PLUS | FmtFlags::ZERO));
        assert_eq!(s.width, Some(8));
        assert_eq!(s.precision, Some(3));
        assert_eq!(s.verb, b'f');
        assert_eq!(i, 7);
    }

    #[test]
    fn bare_dot_is_precision_zero() {
        let (s, _) = parse(".s").unwrap();
        assert_eq!(s.precision, Some(0));
        assert_eq!(s.verb, b's');
    }

    #[test]
    fn bails() {
        assert!(parse("*d").is_none()); // arg-driven width
        assert!(parse("5.*f").is_none()); // arg-driven precision
        assert!(parse("99999d").is_none()); // over MAX_FIELD
        assert!(parse("5").is_none()); // missing verb
    }
}
