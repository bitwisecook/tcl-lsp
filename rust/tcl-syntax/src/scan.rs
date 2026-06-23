//! Tcl `scan` conversion-specifier grammar — the shared spec parser.
//!
//! The companion to [`crate::format`]: where that parses a `format` (printf)
//! conversion, this parses one `scan` (scanf) conversion
//! (`%[*][n$][width][size]verb`) into a structured [`ScanConversion`] — the
//! parsed item every consumer builds on, rather than each re-scanning the
//! string: the runtime matcher (`tcl_cmd_core::scan`), its up-front validator,
//! and the LSP/compiler's inline-`scan` const-folder (`tcl-registry`). Matching
//! and folding are value-/dialect-specific and stay with each consumer; this is
//! the one place the *grammar* lives (reference Tcl 9.0 `Tcl_ScanObjCmd` /
//! `ValidateFormat`, `tmp/tcl9.0.3/generic/tclScan.c`).
//!
//! [`parse_conversion`] both parses and validates a single specifier (rejecting
//! a width on `%c`, a size modifier on `%c`/`%n`/`%s`/`%[`, an unterminated
//! `%[`, and an unknown verb); the cross-specifier rules (mixing `%`/`%n$`,
//! the `%n$` index range, and the variable/specifier count) depend on the call
//! and stay with the runtime.

/// A C size modifier on a scan conversion. Tcl treats every integer as a wide,
/// so these are parsed and then discarded by the matcher; they survive here only
/// because a "longer"/"big" modifier on `%c`/`%n`/`%s`/`%[` is an *error*.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeModifier {
    /// `h` — short. (No effect, and never an error.)
    Short,
    /// `l` / `j` / `q` / `z` / `t` — "longer".
    Long,
    /// `ll` / `L` — "big" (bignum).
    Big,
}

impl SizeModifier {
    /// Whether this modifier is a "longer"/"big" form — the kind that is invalid
    /// on `%c`/`%n`/`%s`/`%[` (C's `SCAN_LONGER | SCAN_BIG`).
    #[must_use]
    pub fn is_wide(self) -> bool {
        matches!(self, SizeModifier::Long | SizeModifier::Big)
    }
}

/// A `%[...]` character set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharSet {
    /// `^` — match characters *not* in the set.
    pub negated: bool,
    /// Literal members (a leading `]`, after the optional `^`, is one of these).
    pub members: Vec<char>,
    /// Inclusive ranges `a-z`.
    pub ranges: Vec<(char, char)>,
}

impl CharSet {
    /// Whether `c` is a member (before applying [`negated`](Self::negated)).
    #[must_use]
    pub fn contains(&self, c: char) -> bool {
        self.members.contains(&c)
            || self
                .ranges
                .iter()
                .any(|&(a, b)| (a..=b).contains(&c) || (b..=a).contains(&c))
    }

    /// Whether `c` matches the set, accounting for `^` negation — the predicate
    /// the matcher consumes a run by.
    #[must_use]
    pub fn matches(&self, c: char) -> bool {
        self.contains(c) != self.negated
    }
}

/// One parsed `scan` conversion specifier (`%[*][n$][width][size]verb`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanConversion {
    /// `*` — match but suppress assignment.
    pub suppress: bool,
    /// `n$` — the 1-based XPG positional argument index (as written).
    pub xpg_index: Option<u32>,
    /// Field width, if present (`0` is kept as written).
    pub width: Option<usize>,
    /// Size modifier (parsed, discarded by the matcher).
    pub size: Option<SizeModifier>,
    /// The conversion verb (`d`/`s`/`x`/`c`/`[`/`n`/`%`/…). `%` denotes the `%%`
    /// literal-percent conversion.
    pub verb: char,
    /// For `%[...]`, the parsed character set.
    pub charset: Option<CharSet>,
}

/// Why a single scan conversion specifier is invalid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanSpecError {
    /// A `%[...]` set was not terminated by `]`.
    UnmatchedBracket,
    /// `%c` was given a field width.
    WidthOnChar,
    /// A "longer"/"big" size modifier on `%c`/`%n`/`%s`/`%[` (carries the verb).
    SizeModifier(char),
    /// An unrecognised conversion character (carries it; `'\0'` at end of input).
    BadConversion(char),
}

impl ScanSpecError {
    /// The Tcl diagnostic message for this error (matching `tclScan.c`), shared
    /// by every consumer so the runtime and the LSP agree verbatim.
    #[must_use]
    pub fn message(&self) -> String {
        match self {
            ScanSpecError::UnmatchedBracket => "unmatched [ in format string".to_string(),
            ScanSpecError::WidthOnChar => {
                "field width may not be specified in %c conversion".to_string()
            }
            ScanSpecError::SizeModifier(v) => {
                format!("field size modifier may not be specified in %{v} conversion")
            }
            ScanSpecError::BadConversion(c) => {
                let shown = if *c == '\0' {
                    String::new()
                } else {
                    c.to_string()
                };
                format!("bad scan conversion character \"{shown}\"")
            }
        }
    }
}

/// Parse one conversion specifier, starting just past the `%` and advancing `i`
/// past the whole specifier (verb and, for `%[`, the closing `]`). The `%%`
/// literal-percent conversion parses to `verb == '%'`.
///
/// # Errors
/// Returns a [`ScanSpecError`] for a width on `%c`, a wide size modifier on
/// `%c`/`%n`/`%s`/`%[`, an unterminated `%[`, or an unknown verb.
pub fn parse_conversion(fmt: &[char], i: &mut usize) -> Result<ScanConversion, ScanSpecError> {
    let flen = fmt.len();
    let peek = |k: usize| if k < flen { fmt[k] } else { '\0' };

    // `%%` — a literal percent (no flags/width/size).
    if peek(*i) == '%' {
        *i += 1;
        return Ok(ScanConversion {
            suppress: false,
            xpg_index: None,
            width: None,
            size: None,
            verb: '%',
            charset: None,
        });
    }

    let mut suppress = false;
    let mut xpg_index = None;

    if peek(*i) == '*' {
        suppress = true;
        *i += 1;
    } else if peek(*i).is_ascii_digit() {
        // Possible XPG `%n$`. The digit run is the index only when a `$` follows;
        // otherwise it is a width, left in place to be parsed below.
        let (val, end) = decimal(fmt, *i);
        if peek(end) == '$' {
            xpg_index = Some(u32::try_from(val).unwrap_or(u32::MAX));
            *i = end + 1;
        }
    }

    // Width.
    let width = if peek(*i).is_ascii_digit() {
        let (val, end) = decimal(fmt, *i);
        *i = end;
        Some(usize::try_from(val).unwrap_or(usize::MAX))
    } else {
        None
    };

    // Size modifier (`hh` is not accepted; a second `h` falls to the verb).
    let size = match peek(*i) {
        'z' | 't' | 'j' | 'q' => {
            *i += 1;
            Some(SizeModifier::Long)
        }
        'L' => {
            *i += 1;
            Some(SizeModifier::Big)
        }
        'l' => {
            *i += 1;
            if peek(*i) == 'l' {
                *i += 1;
                Some(SizeModifier::Big)
            } else {
                Some(SizeModifier::Long)
            }
        }
        'h' => {
            *i += 1;
            Some(SizeModifier::Short)
        }
        _ => None,
    };
    let wide = size.is_some_and(SizeModifier::is_wide);

    let verb = peek(*i);
    let conv = |charset| ScanConversion {
        suppress,
        xpg_index,
        width,
        size,
        verb,
        charset,
    };
    match verb {
        'c' => {
            if width.is_some() {
                return Err(ScanSpecError::WidthOnChar);
            }
            if wide {
                return Err(ScanSpecError::SizeModifier('c'));
            }
            *i += 1;
            Ok(conv(None))
        }
        'n' | 's' => {
            if wide {
                return Err(ScanSpecError::SizeModifier(verb));
            }
            *i += 1;
            Ok(conv(None))
        }
        'd' | 'e' | 'E' | 'f' | 'g' | 'G' | 'i' | 'o' | 'x' | 'X' | 'b' | 'u' => {
            *i += 1;
            Ok(conv(None))
        }
        '[' => {
            if wide {
                return Err(ScanSpecError::SizeModifier('['));
            }
            *i += 1; // past `[`
            let charset = parse_charset(fmt, i)?;
            Ok(conv(Some(charset)))
        }
        other => Err(ScanSpecError::BadConversion(other)),
    }
}

/// Parse a `%[...]` set body, with `i` just past the `[`, advancing past the
/// closing `]`. A `]` immediately after `[` (or `[^`) is a literal member.
fn parse_charset(fmt: &[char], i: &mut usize) -> Result<CharSet, ScanSpecError> {
    let flen = fmt.len();
    let mut negated = false;
    if *i < flen && fmt[*i] == '^' {
        negated = true;
        *i += 1;
    }
    let mut members = Vec::new();
    let mut ranges = Vec::new();
    let mut first = true;
    loop {
        if *i >= flen {
            return Err(ScanSpecError::UnmatchedBracket);
        }
        let c = fmt[*i];
        if c == ']' && !first {
            *i += 1;
            return Ok(CharSet {
                negated,
                members,
                ranges,
            });
        }
        first = false;
        // A range `a-z`, but not a trailing `-` or a `-]`.
        if *i + 2 < flen && fmt[*i + 1] == '-' && fmt[*i + 2] != ']' {
            ranges.push((c, fmt[*i + 2]));
            *i += 3;
        } else {
            members.push(c);
            *i += 1;
        }
    }
}

/// Parse a run of decimal digits starting at `start`, returning `(value,
/// end_index)`. Saturates rather than overflowing (a field this large is
/// rejected elsewhere).
fn decimal(fmt: &[char], start: usize) -> (u64, usize) {
    let mut j = start;
    let mut val: u64 = 0;
    while j < fmt.len() && fmt[j].is_ascii_digit() {
        val = val
            .saturating_mul(10)
            .saturating_add(u64::from(fmt[j].to_digit(10).unwrap()));
        j += 1;
    }
    (val, j)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Result<(ScanConversion, usize), ScanSpecError> {
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        let c = parse_conversion(&chars, &mut i)?;
        Ok((c, i))
    }

    #[test]
    fn plain_and_modifiers() {
        let (c, i) = parse("d").unwrap();
        assert_eq!(c.verb, 'd');
        assert!(!c.suppress);
        assert_eq!(c.width, None);
        assert_eq!(i, 1);

        let (c, i) = parse("*12ld").unwrap();
        assert!(c.suppress);
        assert_eq!(c.width, Some(12));
        assert_eq!(c.size, Some(SizeModifier::Long));
        assert_eq!(c.verb, 'd');
        assert_eq!(i, 5);

        let (c, _) = parse("3$s").unwrap();
        assert_eq!(c.xpg_index, Some(3));
        assert_eq!(c.verb, 's');

        assert_eq!(parse("%").unwrap().0.verb, '%');
    }

    #[test]
    fn charsets() {
        let (c, i) = parse("[a-cx]").unwrap();
        let set = c.charset.unwrap();
        assert!(!set.negated);
        assert_eq!(set.ranges, vec![('a', 'c')]);
        assert_eq!(set.members, vec!['x']);
        assert_eq!(i, 6);
        assert!(set.matches('b'));
        assert!(set.matches('x'));
        assert!(!set.matches('z'));

        // A leading `]` (after `^`) is a literal member, not the terminator.
        let set = parse("[^]a]").unwrap().0.charset.unwrap();
        assert!(set.negated);
        assert_eq!(set.members, vec![']', 'a']);
    }

    #[test]
    fn spec_errors() {
        assert_eq!(parse("10c"), Err(ScanSpecError::WidthOnChar));
        assert_eq!(parse("ls"), Err(ScanSpecError::SizeModifier('s')));
        assert_eq!(parse("[a"), Err(ScanSpecError::UnmatchedBracket));
        assert_eq!(parse("[^a"), Err(ScanSpecError::UnmatchedBracket));
        assert_eq!(parse("[]a"), Err(ScanSpecError::UnmatchedBracket));
        assert_eq!(parse("D"), Err(ScanSpecError::BadConversion('D')));
        // `%hd` is fine — `h` is a (discarded) short modifier, not a wide one.
        assert_eq!(parse("hd").unwrap().0.verb, 'd');
    }
}
