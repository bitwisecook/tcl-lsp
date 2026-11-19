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
//! `ValidateFormat`, `tmp/tcl9.0.4/generic/tclScan.c`).
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
    parse_conversion_with_offsets(fmt, i, &mut ConversionOffsets::default(), false)
}

#[derive(Default)]
struct ConversionOffsets {
    width_start: usize,
    size_start: usize,
    verb_start: usize,
}

/// Byte extents in a scan format *value*, before any mapping to a source word.
#[derive(Debug, PartialEq, Eq)]
pub struct ConversionSpan {
    /// The introducing percent sign.
    pub percent: std::ops::Range<usize>,
    /// The `*` or `n$` after the percent, if present.
    pub assignment: std::ops::Range<usize>,
    /// The decimal width and size modifier, each empty when absent.
    pub width: std::ops::Range<usize>,
    /// The modifier as written, distinguishing `l` from `ll`.
    pub size: std::ops::Range<usize>,
    /// The verb, including the full bracket expression for a scanset.
    pub conversion: std::ops::Range<usize>,
    /// The parsed conversion and its operand-consuming behaviour.
    pub spec: ScanConversion,
}

/// Read value-bearing directives up to the first invalid or unproved conversion.
/// Literal `%%` consumes its two bytes without introducing a field. This is
/// per-directive syntax; positional agreement and variable arity belong to the call.
#[must_use]
pub fn conversion_spans(value: &str, profile: &tcl_dialect::DialectProfile) -> Vec<ConversionSpan> {
    use tcl_dialect::{TclVersion, model::Family};
    let version = profile.runtime_version();
    let jim = profile.runtime_core_family() == Some(Family::Jim);
    // Counted and NUL-terminated template readers can diverge after this point.
    let value = value.split_once('\0').map_or(value, |(prefix, _)| prefix);
    let (mut offsets, chars): (Vec<_>, Vec<_>) = value.char_indices().unzip();
    offsets.push(value.len());
    let mut fields = Vec::new();
    let mut at = 0;
    while let Some(&ch) = chars.get(at) {
        let start = at;
        at += 1;
        if ch != '%' {
            continue;
        }
        let mut parts = ConversionOffsets::default();
        let Ok(spec) = parse_conversion_with_offsets(&chars, &mut at, &mut parts, jim) else {
            break;
        };
        if spec.verb == '%' {
            continue;
        }
        let size = offsets[parts.size_start]..offsets[parts.verb_start];
        let admitted = match &value[size.clone()] {
            "j" | "q" | "z" | "t" => version >= Some(TclVersion::V9_0),
            "ll" => {
                version >= Some(TclVersion::V8_5)
                    && (spec.verb != 'u' || version >= Some(TclVersion::V9_0))
            }
            "l" | "L" if spec.verb == 'u' => version.is_some(),
            "h" if spec.verb == '[' => version.is_some(),
            _ => true,
        } && match spec.verb {
            'b' => version >= Some(TclVersion::V8_6),
            'E' | 'G' | 'X' => version >= Some(TclVersion::V8_5),
            _ => true,
        };
        if !admitted
            || spec.xpg_index == Some(0)
            || spec.width.is_some_and(|w| {
                w >= i32::MAX as usize && version < Some(TclVersion::V9_0)
                    || w >= isize::MAX as usize
            })
        {
            break;
        }
        fields.push(ConversionSpan {
            percent: offsets[start]..offsets[start + 1],
            assignment: offsets[start + 1]..offsets[parts.width_start],
            width: offsets[parts.width_start]..offsets[parts.size_start],
            size,
            conversion: offsets[parts.verb_start]..offsets[at],
            spec,
        });
    }
    fields
}

fn parse_conversion_with_offsets(
    fmt: &[char],
    i: &mut usize,
    offsets: &mut ConversionOffsets,
    jim: bool,
) -> Result<ScanConversion, ScanSpecError> {
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

    offsets.width_start = *i;
    // Width.
    let width = if peek(*i).is_ascii_digit() {
        let (val, end) = decimal(fmt, *i);
        *i = end;
        Some(usize::try_from(val).unwrap_or(usize::MAX))
    } else {
        None
    };

    offsets.size_start = *i;
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
    offsets.verb_start = *i;
    let wide = !jim && size.is_some_and(SizeModifier::is_wide);

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
            if width.is_some_and(|w| !jim || w != 0) {
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
            if wide || (jim && size.is_some()) {
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
/// Scan a `scan` %-string for version-gated conversions — the §6
/// argument-DSL rung. Modelled (evidence-bounded): `%b` (binary), added
/// in Tcl 8.6. Returns `(char_offset, feature, min)` per use; the caller
/// compares against the dialect's effective Tcl version. Unparseable
/// conversions contribute nothing.
#[must_use]
pub fn version_gated_uses(fmt: &str) -> Vec<(usize, &'static str, tcl_dialect::TclVersion)> {
    use tcl_dialect::TclVersion;
    let chars: Vec<char> = fmt.chars().collect();
    let mut uses = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if chars.get(i) == Some(&'%') {
            i += 1;
            continue;
        }
        let Ok(conv) = parse_conversion(&chars, &mut i) else {
            i = start + 1;
            continue;
        };
        if conv.verb == 'b' {
            uses.push((start, "%b binary conversion", TclVersion::V8_6));
        }
    }
    uses
}

#[cfg(test)]
mod spans_tests {
    use super::*;

    #[test]
    fn scan_spans_retain_the_selected_grammar_and_original_byte_offsets() {
        use tcl_dialect::model::{DialectPoint, Release};
        let jim = tcl_dialect::DialectProfile::projected_from_point(
            "jim",
            &[],
            "Jim",
            DialectPoint::canonical(Release::JIM_0_84),
        );
        let source = "é%%d %*3d %1$hx %[a-z] %n";
        for (name, uppercase, binary, extended) in [
            ("tcl8.4", false, false, false),
            ("tcl8.5", true, false, false),
            ("tcl8.6", true, true, false),
            ("tcl9.0", true, true, true),
            ("tcl9.1", true, true, true),
            ("jim", false, false, false),
        ] {
            let profile = if name == "jim" {
                &jim
            } else {
                tcl_dialect::DialectProfile::find(name).unwrap()
            };
            let fields = conversion_spans(source, profile);
            assert_eq!(fields.len(), 4, "{name}");
            assert_eq!(&source[fields[0].assignment.clone()], "*");
            assert_eq!(&source[fields[0].width.clone()], "3");
            assert_eq!(&source[fields[1].assignment.clone()], "1$");
            assert_eq!(&source[fields[1].size.clone()], "h");
            assert_eq!(&source[fields[2].conversion.clone()], "[a-z]");
            assert_eq!(fields[3].spec.verb, 'n');
            for (template, accepted) in [
                ("%E", uppercase),
                ("%b", binary),
                ("%jd", extended),
                ("%#d", false),
            ] {
                assert_eq!(
                    !conversion_spans(template, profile).is_empty(),
                    accepted,
                    "{name} {template}"
                );
            }
        }
    }
}
