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

//! Tcl `format` conversion-specifier grammar — the shared spec parser.
//!
//! Parses one `%…` conversion (`%[flags][width][.precision]verb`) into a
//! [`Spec`], the structured form both consumers render from: the LSP/compiler's
//! version-aware const-folder (`tcl-registry`) and the WASM runtime's renderer
//! over its own value type. Rendering is **not** here — it is value-type- and
//! dialect-specific, so each consumer owns it; this module is the one place the
//! specifier *grammar* lives (reference Tcl 9.0 `Tcl_AppendFormatToObj`,
//! `tmp/tcl9.0.3/generic/tclStringObj.c`).
//!
//! Arg-driven `*` width/`.*` precision parse into `width_star`/`precision_star`
//! (the runtime renderer consumes a leading argument; the const-folder declines
//! them). The modelled subset still bails (`None`) on an over-[`MAX_FIELD`]
//! field, positional `%n$`, and unknown size modifiers — a missed parse is
//! never wrong for a const-fold, and the runtime can extend it.

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
    /// The width is `*` — taken from an argument at render time (consumed before
    /// the value). A negative argument left-justifies (sets `MINUS`).
    pub width_star: bool,
    /// The precision is `.*` — taken from an argument at render time.
    pub precision_star: bool,
    /// A 1-based positional argument selector (`%n$…`), if present: the
    /// conversion draws its value from `args[n-1]` instead of the next
    /// sequential argument (`format {%2$d-%1$d} 10 20` → `20-10`). `None` for
    /// the ordinary sequential form.
    pub arg_index: Option<usize>,
    /// A `ll` / `L` bignum size modifier was present. Version-relevant:
    /// the `ll`+`u` combination ("unsigned bignum") raises before Tcl 9.0
    /// (oracle-verified: tclsh8.6 `format %llu 5` → "unsigned bignum
    /// format is invalid"; tclsh9.0.4 → `5`).
    pub big: bool,
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
    // Optional positional selector `n$` (1-based), right after the `%` and
    // before any flags: `%2$d` draws from the 2nd argument. A digit run *not*
    // followed by `$` is an ordinary width, so only commit when the `$` is
    // present (otherwise leave `i` untouched for the width parse below).
    let arg_index = parse_arg_index(fmt, i);
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
    // `*` width: take it from an argument at render time.
    let width_star = fmt.get(*i) == Some(&b'*');
    let width = if width_star {
        *i += 1;
        None
    } else {
        match parse_field(fmt, i)? {
            Field::Absent => None,
            Field::Size(n) => Some(n),
        }
    };
    let mut precision_star = false;
    let precision = if fmt.get(*i) == Some(&b'.') {
        *i += 1;
        if fmt.get(*i) == Some(&b'*') {
            *i += 1;
            precision_star = true;
            None
        } else {
            // a `.` with no digits means precision 0
            Some(match parse_field(fmt, i)? {
                Field::Absent => 0,
                Field::Size(n) => n,
            })
        }
    } else {
        None
    };
    // C size modifiers: Tcl treats every integer as a wide, so these are parsed
    // and discarded (`format %ld 5` → `5`). `l`/`ll`, or a single
    // `h`/`j`/`z`/`q`/`t`/`L`. `hh` is *not* accepted — the second `h` is left to
    // fail as the verb (`format %hhd` → `bad field specifier "h"`, matching C).
    let mut big = false;
    match fmt.get(*i) {
        Some(b'l') => {
            *i += 1;
            if fmt.get(*i) == Some(&b'l') {
                *i += 1;
                big = true;
            }
        }
        Some(b'L') => {
            *i += 1;
            big = true;
        }
        Some(b'h' | b'j' | b'z' | b'q' | b't') => *i += 1,
        _ => {}
    }
    let verb = *fmt.get(*i)?;
    *i += 1;
    Some(Spec {
        flags,
        width,
        precision,
        verb,
        width_star,
        precision_star,
        arg_index,
        big,
    })
}

/// Parse an optional positional selector `n$` (1-based) at `*i`. Advances `i`
/// past `n$` and returns `Some(n)` only when a digit run is immediately
/// followed by `$`; otherwise leaves `i` unchanged (the digits are a width).
fn parse_arg_index(fmt: &[u8], i: &mut usize) -> Option<usize> {
    let mut j = *i;
    let mut n = 0usize;
    while let Some(&d) = fmt.get(j) {
        if !d.is_ascii_digit() {
            break;
        }
        n = n.checked_mul(10)?.checked_add(usize::from(d - b'0'))?;
        j += 1;
    }
    if j > *i && fmt.get(j) == Some(&b'$') {
        *i = j + 1;
        Some(n)
    } else {
        None
    }
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
/// One version-gated feature used in a format string — the §6
/// argument-DSL rung (dialect-profile-model.md): the conversion parses
/// everywhere, but *raises at runtime* below `min`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionGatedUse {
    /// Byte offset of the `%` that starts the conversion, within the
    /// format string.
    pub offset: usize,
    /// Human-readable feature name (`"%b binary conversion"`).
    pub feature: &'static str,
    /// The lowest Tcl release that accepts it.
    pub min: tcl_dialect::TclVersion,
}

/// Scan a `format` %-string for version-gated conversions, reporting every
/// use with its introducing release — the caller compares against the
/// dialect's effective Tcl version.
///
/// Modelled (evidence-bounded):
/// - `%b` (binary) — added in Tcl 8.6 (raises "bad field specifier" on
///   8.4/8.5).
/// - the `ll`/`L` + `u` combination — unsigned bignum, Tcl 9.0+
///   (oracle-verified: tclsh8.6 `format %llu 5` → "unsigned bignum format
///   is invalid"; tclsh9.0.4 → `5`).
///
/// Unparseable conversions contribute nothing — a missed check is never
/// a false positive.
#[must_use]
pub fn version_gated_uses(fmt: &str) -> Vec<VersionGatedUse> {
    use tcl_dialect::TclVersion;
    let bytes = fmt.as_bytes();
    let mut uses = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if bytes.get(i) == Some(&b'%') {
            i += 1;
            continue;
        }
        let Some(spec) = parse_spec(bytes, &mut i) else {
            // Unparseable — skip a byte and resynchronise on the next `%`.
            i = start + 1;
            continue;
        };
        if spec.verb == b'b' {
            uses.push(VersionGatedUse {
                offset: start,
                feature: "%b binary conversion",
                min: TclVersion::V8_6,
            });
        }
        if spec.big && spec.verb == b'u' {
            uses.push(VersionGatedUse {
                offset: start,
                feature: "%llu unsigned bignum conversion",
                min: TclVersion::V9_0,
            });
        }
    }
    uses
}
