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
//! `tmp/tcl9.0.4/generic/tclStringObj.c`).
//!
//! Arg-driven `*` width/`.*` precision parse into `width_star`/`precision_star`
//! (the runtime renderer consumes a leading argument; the const-folder declines
//! them), and a positional `%n$` selector into `arg_index`. The modelled subset
//! still bails (`None`) on an over-[`MAX_FIELD`] field and on unknown size
//! modifiers — a missed parse is never wrong for a const-fold, and the runtime
//! can extend it.

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

/// A C size modifier on a `format` conversion.
///
/// The spelling is preserved even where Tcl gives multiple modifiers the same
/// effective width. Consumers that do not model the modifier's coercion must
/// therefore decline the conversion rather than accidentally treating it as
/// an unmodified one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeModifier {
    /// `h` — truncate an integer conversion to C `short` width.
    Short,
    /// `l` — use Tcl's wide-integer path.
    Long,
    /// `ll` — use Tcl's bignum path.
    LongLong,
    /// `j` — use Tcl's wide-integer path.
    IntMax,
    /// `z` — use pointer-sized integer width.
    Size,
    /// `q` — use Tcl's wide-integer path.
    Quad,
    /// `t` — use pointer-sized integer width.
    PtrDiff,
    /// `L` — use Tcl's bignum path.
    Big,
}

/// The integer width an integer conversion renders through, once the size
/// modifier and the release are both known.
///
/// C Tcl's rule is one rule: take the value modulo 2^width, then read those
/// bits **signed** for `d`/`i` and **unsigned** for `u`/`x`/`X`/`o`/`b`.
/// Measured on tclsh 8.4.20/8.5.19/8.6.18/9.0.4/9.1b0 —
/// `format %hd 5000000000` is `-3584` and `format %hu 5000000000` is `61952`
/// on every one of them, `format %hd 32768` is `-32768`, and
/// `format %d 4294967296` is `4294967296` on 8.x but `0` on 9.x.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerWidth {
    /// C `short`, from an `h` modifier. 16 bits on every platform Tcl
    /// supports, so this one needs no platform axis.
    Short,
    /// C `int` — Tcl 9's width for an unmodified integer conversion. 32 bits
    /// on every platform Tcl supports.
    Int,
    /// 64 bits: `Tcl_WideInt`, and the unmodified width before Tcl 9.
    Wide,
}

impl IntegerWidth {
    /// The mask selecting this width's low bits, or `None` for the full
    /// 64, whose mask does not fit a positive `i64`.
    const fn low_mask(self) -> Option<i64> {
        match self {
            Self::Short => Some(0xFFFF),
            Self::Int => Some(0xFFFF_FFFF),
            Self::Wide => None,
        }
    }

    /// The value's low bits, read as a signed integer of this width.
    ///
    /// Masked and sign-extended arithmetically rather than cast: a narrowing
    /// cast is what `clippy::pedantic` rejects, and the arithmetic says what
    /// is meant.
    #[must_use]
    pub fn signed(self, value: i64) -> i64 {
        let Some(mask) = self.low_mask() else {
            return value;
        };
        let sign_bit = (mask >> 1) + 1;
        let low = value & mask;
        if low & sign_bit == 0 {
            low
        } else {
            low - mask - 1
        }
    }

    /// The value's low bits, read as an unsigned integer of this width.
    #[must_use]
    pub fn unsigned(self, value: i64) -> u64 {
        // The two's-complement bit pattern, without a sign-losing cast.
        let bits = u64::from_ne_bytes(value.to_ne_bytes());
        match self.low_mask() {
            Some(mask) => bits & u64::from_ne_bytes(mask.to_ne_bytes()),
            None => bits,
        }
    }
}

/// The width `size` selects for an integer conversion under `syntax`.
///
/// The one owner of this policy, so the renderers cannot drift apart on it.
///
/// **Two families are deliberately answered [`IntegerWidth::Wide`] rather than
/// correctly**, because answering them needs facts this crate does not have:
///
/// * `ll` and `L` select Tcl's **bignum** path, not a 64-bit one —
///   `format %lld 9223372036854775808` prints `9223372036854775808` where
///   `%ld` prints `-9223372036854775808`. A renderer whose value type is
///   `i64` has already lost the distinction before it gets here.
/// * `l` before Tcl 9, `z` and `t` are **platform**-dependent (C `long`,
///   `TCL_WIDE_INT_IS_LONG`, and `sizeof(void *) > sizeof(int)` respectively).
///   `Wide` is right on LP64 and wrong on an ILP32 build; `tcl-dialect` has no
///   platform axis to ask.
///
/// Both are recorded as gaps rather than guessed at; the unmodified and `h`
/// answers below are determined on every platform Tcl supports.
#[must_use]
pub fn integer_width(
    size: Option<SizeModifier>,
    syntax: tcl_dialect::NumberSyntax,
) -> IntegerWidth {
    match size {
        Some(SizeModifier::Short) => IntegerWidth::Short,
        Some(
            SizeModifier::Long
            | SizeModifier::LongLong
            | SizeModifier::IntMax
            | SizeModifier::Size
            | SizeModifier::Quad
            | SizeModifier::PtrDiff
            | SizeModifier::Big,
        ) => IntegerWidth::Wide,
        // Unmodified: Tcl 9 renders through C `int`, 8.x through the wide
        // path. `NumberSyntax::Tcl90` is this module's existing discriminator
        // for "Tcl 9 or later" (it already decides the `%#d` -> `0d` prefix);
        // Jim keeps the pre-9 answer, since no jimsh oracle was available to
        // establish otherwise.
        None => match syntax {
            tcl_dialect::NumberSyntax::Tcl90 => IntegerWidth::Int,
            _ => IntegerWidth::Wide,
        },
    }
}

impl SizeModifier {
    /// Whether this modifier selects Tcl's bignum formatting path.
    #[must_use]
    pub fn is_big(self) -> bool {
        matches!(self, Self::LongLong | Self::Big)
    }

    /// Tcl release surface that first accepts this spelling.
    ///
    /// Tcl 8.4 accepts `h` and a single `l`. Tcl 8.5 adds `ll`; Tcl 9.0
    /// adds the C99/POSIX spellings `j`, `z`, `q`, `t`, and `L`.
    const fn surface(self) -> &'static [SpecSurface] {
        match self {
            Self::Short | Self::Long => &[],
            Self::LongLong => SpecSurface::TCL85_PLUS,
            Self::IntMax | Self::Size | Self::Quad | Self::PtrDiff | Self::Big => {
                SpecSurface::TCL90_PLUS
            }
        }
    }

    /// Lowest Tcl release that accepts this spelling, when gated.
    const fn minimum_version(self) -> Option<tcl_dialect::TclVersion> {
        match self {
            Self::Short | Self::Long => None,
            Self::LongLong => Some(tcl_dialect::TclVersion::V8_5),
            Self::IntMax | Self::Size | Self::Quad | Self::PtrDiff | Self::Big => {
                Some(tcl_dialect::TclVersion::V9_0)
            }
        }
    }

    /// Stable diagnostic spelling for this modifier lifecycle.
    const fn feature(self) -> &'static str {
        match self {
            Self::Short => "%h size modifier",
            Self::Long => "%l size modifier",
            Self::LongLong => "%ll size modifier",
            Self::IntMax => "%j size modifier",
            Self::Size => "%z size modifier",
            Self::Quad => "%q size modifier",
            Self::PtrDiff => "%t size modifier",
            Self::Big => "%L size modifier",
        }
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
    ///
    /// **`Some(0)` is reachable**, and deliberately so: `%0$d` is grammatical
    /// — C Tcl parses the selector and then rejects the *index*, with
    /// `"%n$" argument index out of range` (tclsh8.6.18 / tclsh9.0.4,
    /// `format {%0$d} a b`). Refusing it here would instead re-read `0` as a
    /// zero-pad flag and leave `$` as the verb, reporting `bad field
    /// specifier` — a different error for the same input. So the 1-based →
    /// 0-based conversion is the consumer's, and a consumer that subtracts
    /// must do so checked; `tcl_cmd_core::format` raises C Tcl's own message
    /// on the underflow.
    pub arg_index: Option<usize>,
    /// The C size modifier, if present. A `ll` / `L` modifier selects Tcl's
    /// bignum path; in particular, its combination with `u` raises before Tcl
    /// 9.0 (oracle-verified: tclsh8.6 `format %llu 5` → "unsigned bignum
    /// format is invalid"; tclsh9.0.4 → `5`).
    pub size: Option<SizeModifier>,
}

/// Conversion verbs implemented by the shared `format` renderer.
#[must_use]
pub fn is_verb(verb: u8) -> bool {
    matches!(
        verb,
        b'b' | b'c'
            | b'd'
            | b'e'
            | b'E'
            | b'f'
            | b'g'
            | b'G'
            | b'i'
            | b'o'
            | b'p'
            | b's'
            | b'u'
            | b'x'
            | b'X'
            | b'%'
    )
}

use tcl_dialect::model::{SpecSurface, surface_admits};

/// Whether a parsed conversion is available under the resolved dialect
/// profile.  Unknown-release profiles abstain rather than guessing.
#[must_use]
pub fn is_available(spec: &Spec, profile: &tcl_dialect::DialectProfile) -> bool {
    let admits = |required: &'static [SpecSurface]| {
        required.is_empty()
            || (profile.runtime_base.is_some()
                && surface_admits(required, Some(&profile.surface_query())))
    };
    let verb_surface = match spec.verb {
        b'b' => SpecSurface::TCL86_PLUS,
        // `%p` was added with Tcl 9's extended format conversions; Tcl 8.4
        // rejects it as a bad field specifier.
        b'p' => SpecSurface::TCL90_PLUS,
        _ => &[],
    };
    let size_surface = spec.size.map_or(&[][..], SizeModifier::surface);
    let unsigned_big_surface = if spec.size.is_some_and(SizeModifier::is_big) && spec.verb == b'u' {
        SpecSurface::TCL90_PLUS
    } else {
        &[]
    };
    // Release-invariant verbs remain recognisable for the permissive
    // profile; a version-gated verb must abstain until a release is resolved.
    admits(verb_surface) && admits(size_surface) && admits(unsigned_big_surface)
}

/// The outcome of parsing a width / `.precision` field.
enum Field {
    /// No digits were present.
    Absent,
    /// A parsed field size.
    Size(usize),
}

/// Parse one conversion's selector / flags / width / `.precision` / size / verb,
/// starting just past the `%` and advancing `i` past the verb. Bails on an
/// over-[`MAX_FIELD`] field or a missing verb.
pub fn parse_spec(fmt: &[u8], i: &mut usize) -> Option<Spec> {
    parse_spec_with_limit(fmt, i, MAX_FIELD)
}

/// Parse a conversion with an explicit field-size ceiling. Renderers use the
/// Tcl value limit; conservative constant-folders retain `MAX_FIELD`.
pub fn parse_spec_with_limit(fmt: &[u8], i: &mut usize, max_field: usize) -> Option<Spec> {
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
        match parse_field(fmt, i) {
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
            Some(match parse_field(fmt, i) {
                Field::Absent => 0,
                Field::Size(n) => n,
            })
        }
    } else {
        None
    };
    // C size modifiers: `l`/`ll`, or a single `h`/`j`/`z`/`q`/`t`/`L`. Keep
    // the exact spelling because the integer coercion width differs. `hh` is
    // *not* accepted — the second `h` is left to fail as the verb (`format
    // %hhd` → `bad field specifier "h"`, matching C).
    let size = match fmt.get(*i) {
        Some(b'l') => {
            *i += 1;
            if fmt.get(*i) == Some(&b'l') {
                *i += 1;
                Some(SizeModifier::LongLong)
            } else {
                Some(SizeModifier::Long)
            }
        }
        Some(b'L') => {
            *i += 1;
            Some(SizeModifier::Big)
        }
        Some(b'h') => {
            *i += 1;
            Some(SizeModifier::Short)
        }
        Some(b'j') => {
            *i += 1;
            Some(SizeModifier::IntMax)
        }
        Some(b'z') => {
            *i += 1;
            Some(SizeModifier::Size)
        }
        Some(b'q') => {
            *i += 1;
            Some(SizeModifier::Quad)
        }
        Some(b't') => {
            *i += 1;
            Some(SizeModifier::PtrDiff)
        }
        _ => None,
    };
    let verb = *fmt.get(*i)?;
    *i += 1;
    let spec = Spec {
        flags,
        width,
        precision,
        verb,
        width_star,
        precision_star,
        arg_index,
        size,
    };
    if spec.width.is_some_and(|n| n > max_field) || spec.precision.is_some_and(|n| n > max_field) {
        None
    } else {
        Some(spec)
    }
}

/// Parse an optional positional selector `n$` (1-based) at `*i`. Advances `i`
/// past `n$` and returns `Some(n)` only when a digit run is immediately
/// followed by `$`; otherwise leaves `i` unchanged (the digits are a width).
///
/// The digit run has no lower bound, so `%0$d` yields `Some(0)` — see
/// [`FormatSpec::arg_index`] for why that is the grammar C Tcl implements and
/// what the consumer owes.
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
/// Saturates on overflow; the caller applies its own field-size ceiling.
fn parse_field(fmt: &[u8], i: &mut usize) -> Field {
    let start = *i;
    let mut n = 0usize;
    while let Some(&d) = fmt.get(*i) {
        if !d.is_ascii_digit() {
            break;
        }
        n = n.saturating_mul(10).saturating_add(usize::from(d - b'0'));
        *i += 1;
    }
    if *i == start {
        Field::Absent
    } else {
        Field::Size(n)
    }
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
/// - `%p` (pointer-style hexadecimal) — added in Tcl 9.0.
/// - `ll` — added in Tcl 8.5; `j`/`z`/`q`/`t`/`L` — Tcl 9.0+.
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
        if spec.verb == b'p' {
            uses.push(VersionGatedUse {
                offset: start,
                feature: "%p pointer conversion",
                min: TclVersion::V9_0,
            });
        }
        if let Some(size) = spec.size
            && let Some(min) = size.minimum_version()
        {
            uses.push(VersionGatedUse {
                offset: start,
                feature: size.feature(),
                min,
            });
        }
        if spec.size.is_some_and(SizeModifier::is_big) && spec.verb == b'u' {
            uses.push(VersionGatedUse {
                offset: start,
                feature: "%llu unsigned bignum conversion",
                min: TclVersion::V9_0,
            });
        }
    }
    uses
}

#[cfg(test)]
mod tests {
    use super::{IntegerWidth, integer_width};
    use tcl_dialect::NumberSyntax;

    /// #1782: the width table, and the two readings of the truncated bits.
    ///
    /// Values are transcripts from real tclsh (8.4.20 / 8.5.19 / 8.6.18 /
    /// 9.0.4 / 9.1b0), which agree on every `h` case and split only on the
    /// unmodified width.
    #[test]
    fn integer_width_follows_the_modifier_and_the_release_issue_1782() {
        // `h` is C `short` on every release — no platform axis needed.
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
            NumberSyntax::Jim,
        ] {
            assert_eq!(
                integer_width(Some(SizeModifier::Short), syntax),
                IntegerWidth::Short,
                "{syntax:?}"
            );
        }

        // Unmodified: `int` from Tcl 9, the wide path before it. Jim keeps
        // the pre-9 answer, since no jimsh oracle established otherwise.
        assert_eq!(integer_width(None, NumberSyntax::Tcl84), IntegerWidth::Wide);
        assert_eq!(integer_width(None, NumberSyntax::Tcl85), IntegerWidth::Wide);
        assert_eq!(integer_width(None, NumberSyntax::Tcl90), IntegerWidth::Int);
        assert_eq!(integer_width(None, NumberSyntax::Jim), IntegerWidth::Wide);
        assert_eq!(
            integer_width(None, NumberSyntax::Jim080),
            IntegerWidth::Wide
        );

        // Every explicit modifier answers `Wide` today — right for `l`/`j`/`q`
        // on LP64, a stated gap for the bignum and pointer-width families.
        for size in [
            SizeModifier::Long,
            SizeModifier::LongLong,
            SizeModifier::IntMax,
            SizeModifier::Size,
            SizeModifier::Quad,
            SizeModifier::PtrDiff,
            SizeModifier::Big,
        ] {
            assert_eq!(
                integer_width(Some(size), NumberSyntax::Tcl90),
                IntegerWidth::Wide,
                "{size:?}"
            );
        }
    }

    /// The same low bits, read two ways — this is what `%hd` and `%hu`
    /// disagreeing about 5000000000 (`-3584` against `61952`) measures.
    #[test]
    fn a_widths_bits_read_signed_and_unsigned_issue_1782() {
        assert_eq!(IntegerWidth::Short.signed(5_000_000_000), -3584);
        assert_eq!(IntegerWidth::Short.unsigned(5_000_000_000), 61952);
        assert_eq!(IntegerWidth::Short.signed(32768), -32768);
        assert_eq!(IntegerWidth::Short.signed(-32769), 32767);

        assert_eq!(IntegerWidth::Int.signed(5_000_000_000), 705_032_704);
        assert_eq!(IntegerWidth::Int.signed(4_294_967_296), 0);
        assert_eq!(IntegerWidth::Int.unsigned(-1), 4_294_967_295);

        assert_eq!(IntegerWidth::Wide.signed(5_000_000_000), 5_000_000_000);
        assert_eq!(IntegerWidth::Wide.unsigned(-1), u64::MAX);
    }

    use super::{
        SizeModifier, Spec, is_available, is_verb, parse_spec, parse_spec_with_limit,
        version_gated_uses,
    };
    use tcl_dialect::DialectProfile;

    #[test]
    fn verb_owner_matches_runtime_surface() {
        assert!(is_verb(b'b'));
        assert!(!is_verb(b'q'));
        assert!(!is_verb(b'a'));
    }

    /// `%0$d` is grammatical: the selector parses, and it is the *index* C
    /// Tcl rejects one layer up. Rejecting it here would re-read `0` as a
    /// zero-pad flag and report `bad field specifier` for a different reason
    /// (#2076).
    #[test]
    fn a_zero_positional_selector_parses_and_is_the_consumers_to_reject() {
        let mut i = 0;
        let spec = parse_spec(b"0$d", &mut i).expect("a zero selector is still a complete spec");
        assert_eq!(spec.arg_index, Some(0));
        assert_eq!(spec.verb, b'd');
        assert_eq!(spec.width, None, "the digits are the selector, not a width");

        // The ordinary form is unchanged, and a digit run with no `$` is
        // still a width.
        let mut i = 0;
        assert_eq!(
            parse_spec(b"2$d", &mut i).expect("positional").arg_index,
            Some(2)
        );
        let mut i = 0;
        let width_only = parse_spec(b"2d", &mut i).expect("width");
        assert_eq!((width_only.arg_index, width_only.width), (None, Some(2)));
    }

    #[test]
    fn field_limits_distinguish_const_fold_and_runtime() {
        let mut i = 0;
        assert!(parse_spec(b"4294967294g", &mut i).is_none());
        let mut i = 0;
        let spec = parse_spec_with_limit(b"4294967294g", &mut i, usize::MAX)
            .expect("runtime accepts Tcl-sized width");
        assert_eq!(
            spec.width,
            Some("4294967294".parse::<usize>().unwrap_or(usize::MAX))
        );
        let mut i = 0;
        let spec = parse_spec_with_limit(b"18446744073709551614g", &mut i, usize::MAX)
            .expect("overflowing width remains a complete spec");
        assert_eq!(
            spec.width,
            Some(
                "18446744073709551614"
                    .parse::<usize>()
                    .unwrap_or(usize::MAX)
            )
        );
        let mut i = 0;
        assert_eq!(
            parse_spec_with_limit(
                b"#d",
                &mut i,
                usize::try_from(i32::MAX).expect("usize is at least 32 bits"),
            )
            .unwrap()
            .verb,
            b'd'
        );
    }

    #[test]
    fn size_modifier_spelling_is_preserved() {
        for (text, expected) in [
            (b"hd".as_slice(), SizeModifier::Short),
            (b"ld".as_slice(), SizeModifier::Long),
            (b"lld".as_slice(), SizeModifier::LongLong),
            (b"jd".as_slice(), SizeModifier::IntMax),
            (b"zd".as_slice(), SizeModifier::Size),
            (b"qd".as_slice(), SizeModifier::Quad),
            (b"td".as_slice(), SizeModifier::PtrDiff),
            (b"Ld".as_slice(), SizeModifier::Big),
        ] {
            let mut i = 0;
            let spec = parse_spec(text, &mut i).expect("size-modified spec parses");
            assert_eq!(
                spec.size,
                Some(expected),
                "{}",
                String::from_utf8_lossy(text)
            );
            assert_eq!(spec.verb, b'd');
            assert_eq!(i, text.len());
        }
        assert!(SizeModifier::LongLong.is_big());
        assert!(SizeModifier::Big.is_big());
        assert!(!SizeModifier::Short.is_big());
    }

    fn parsed(text: &[u8]) -> Spec {
        let mut i = 0;
        parse_spec(text, &mut i).expect("format spec parses")
    }

    #[test]
    fn size_modifier_availability_tracks_tcl_release() {
        let v84 = DialectProfile::find("tcl8.4").expect("Tcl 8.4 profile");
        let v85 = DialectProfile::find("tcl8.5").expect("Tcl 8.5 profile");
        let v86 = DialectProfile::find("tcl8.6").expect("Tcl 8.6 profile");
        let v90 = DialectProfile::find("tcl9.0").expect("Tcl 9.0 profile");

        for text in [b"hd".as_slice(), b"ld".as_slice()] {
            assert!(is_available(&parsed(text), v84));
        }
        assert!(!is_available(&parsed(b"lld"), v84));
        assert!(is_available(&parsed(b"lld"), v85));
        for text in [
            b"jd".as_slice(),
            b"zd".as_slice(),
            b"qd".as_slice(),
            b"td".as_slice(),
            b"Ld".as_slice(),
        ] {
            assert!(!is_available(&parsed(text), v86), "{text:?}");
            assert!(is_available(&parsed(text), v90), "{text:?}");
        }
        assert!(!is_available(&parsed(b"llu"), v86));
        assert!(is_available(&parsed(b"llu"), v90));
        assert!(!is_available(&parsed(b"p"), v86));
        assert!(is_available(&parsed(b"p"), v90));
    }

    #[test]
    fn size_modifier_diagnostics_report_their_own_lifecycle() {
        let uses = version_gated_uses("%hd %ld %b %p %lld %jd %zd %qd %td %Ld");
        assert_eq!(
            uses.iter()
                .map(|use_| (use_.feature, use_.min))
                .collect::<Vec<_>>(),
            vec![
                ("%b binary conversion", tcl_dialect::TclVersion::V8_6),
                ("%p pointer conversion", tcl_dialect::TclVersion::V9_0),
                ("%ll size modifier", tcl_dialect::TclVersion::V8_5),
                ("%j size modifier", tcl_dialect::TclVersion::V9_0),
                ("%z size modifier", tcl_dialect::TclVersion::V9_0),
                ("%q size modifier", tcl_dialect::TclVersion::V9_0),
                ("%t size modifier", tcl_dialect::TclVersion::V9_0),
                ("%L size modifier", tcl_dialect::TclVersion::V9_0),
            ]
        );
    }
}
