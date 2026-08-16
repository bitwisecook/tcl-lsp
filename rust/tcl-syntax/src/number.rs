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

//! Tcl numeric-literal grammar — the shared `TclParseNumber` port.
//!
//! Classifies a string as a Tcl 9.0 number, re-derived from reference
//! `tmp/tcl9.0.3/generic/tclStrToD.c::TclParseNumber`. This is the *parse*
//! direction of the numeric tower (string → number); the *format* direction
//! (number → shortest-round-trip string) and the bignum arithmetic live with
//! each consumer (the runtime over libtommath `mp_int`, the compiler's
//! const-folder over its own value type) — both classify with this one grammar.
//!
//! The output is one of the four tower types ([`Number`]): a wide
//! [`Number::Int`] when the integer fits `i64`, a [`Number::Big`] (sign +
//! radix + cleaned digits, for the consumer to build a bignum from) when it
//! overflows, a [`Number::Double`] for floats and `±Inf`, or a [`Number::Nan`].
//! There is **no** boolean handling here —
//! `true`/`yes`/`on`… are `Tcl_GetBoolean`'s job, not `TclParseNumber`'s.
//!
//! ## Forms, by release
//!
//! The numeric grammar changed twice across the supported range, so the shape
//! accepted here depends on [`ParseFlags::syntax`] (a [`NumberSyntax`], derived
//! from the dialect). Defaulting to 9.0 keeps modern rules for callers that
//! genuinely have no dialect.
//!
//! - Optional sign, optional surrounding whitespace (unless [`ParseFlags::no_whitespace`]).
//! - `0x`/`0X` hex in every release. `0o`/`0O` and `0b`/`0B` from **8.5**;
//!   `0d`/`0D` from **9.0** (case-insensitive, each needs ≥1 following digit).
//!   An unavailable prefix is not a prefix at all, so `0o17` under 8.4 parses as
//!   `0` with `o17` trailing.
//! - A bare leading `0` is **octal up to 8.6** (`0755` == 493) and **decimal
//!   from 9.0** (`0755` == 755). Under the octal rule `08` yields `0` with `8`
//!   trailing, mirroring C's scan stopping at the invalid digit, while a run
//!   followed by `.`/`e` stays a decimal float (`0.5`, `07.5`).
//! - `_` digit separators between same-base digits (`1__000`, `0xff__ff`) from
//!   **9.0**, and never when [`ParseFlags::no_underscore`] is set.
//! - Decimal floats: `1.5`, `.5`, `1.`, `1e9`, `1.5E-3` (no hex/oct/bin floats).
//! - `Inf`/`Infinity` and `NaN`/`NaN(hexpayload)` — case-insensitive.

use std::borrow::Cow;

pub use tcl_dialect::NumberSyntax;

/// Integer radix (base) for a [`Number::Big`]'s digit string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radix {
    /// Base 2 (`0b…`).
    Bin = 2,
    /// Base 8 (`0o…`).
    Oct = 8,
    /// Base 10 (the default / `0d…`).
    Dec = 10,
    /// Base 16 (`0x…`).
    Hex = 16,
}

/// A classified Tcl number — one rung of the numeric tower.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    /// An integer that fits in a wide (`i64`).
    Int(i64),
    /// An integer too large for a wide: build the bignum from `digits` (sign,
    /// radix prefix, and `_` separators already stripped) in `radix`, negating
    /// when `negative`. (E.g. feed `digits` to libtommath `mp_read_radix`.)
    Big {
        /// The value's sign.
        negative: bool,
        /// The base of `digits`.
        radix: Radix,
        /// Magnitude digits in `radix`, cleaned (no sign/prefix/underscores).
        digits: String,
    },
    /// A floating-point value (including `±Inf`).
    Double(f64),
    /// IEEE NaN, with the optional `NaN(hexpayload)` mantissa payload.
    Nan {
        /// Whether the literal was sign-negated (`-NaN`).
        negative: bool,
        /// The `NaN(hex)` payload, if one was given.
        payload: Option<u64>,
    },
}

/// How to parse a number: `TclParseNumber`'s flag bits (the subset used so
/// far) plus the release's numeric-literal grammar.
///
/// [`Default`] is Tcl 9.0 syntax, so a caller that does not know its dialect
/// keeps the modern rules; a version-aware caller sets [`ParseFlags::syntax`]
/// (or builds one with [`ParseFlags::for_syntax`]).
#[derive(Debug, Clone, Copy)]
pub struct ParseFlags {
    /// Reject a fractional part / exponent (`TCL_PARSE_INTEGER_ONLY`).
    pub integer_only: bool,
    /// Reject leading/trailing whitespace (`TCL_PARSE_NO_WHITESPACE`).
    pub no_whitespace: bool,
    /// Reject `_` digit separators (`TCL_PARSE_NO_UNDERSCORE`).
    pub no_underscore: bool,
    /// The release's numeric-literal grammar: which radix prefixes exist,
    /// whether a bare leading `0` is octal, and whether `_` separators are
    /// allowed. See [`NumberSyntax`].
    pub syntax: NumberSyntax,
}

/// The numeric grammar this build of the runtime parses, installed once by the
/// embedder and fixed for its lifetime.
///
/// C has no per-call equivalent: its numeric grammar is a *build-time*
/// constant (`tclStrToD.c` decides octal-by-leading-zero with `#define` /
/// `#undef KILL_OCTAL`), so the release a binary emulates is settled before any
/// script runs. This mirrors that — the runtime is built for one dialect and
/// does not switch during execution, which is why this is ambient state rather
/// than an argument threaded through every `Tcl_GetIntFromObj`-shaped call.
///
/// A caller that must be explicit (a test, or a tool handling several dialects
/// in one process) passes [`ParseFlags::for_syntax`] instead of relying on it.
mod ambient {
    use super::NumberSyntax;
    use std::cell::Cell;

    thread_local! {
        // Spelled out rather than `NumberSyntax::default()` because `Default` is
        // not const-callable; `numbers_default_matches_the_ambient_initialiser`
        // pins the two together so they cannot drift.
        static SYNTAX: Cell<NumberSyntax> = const { Cell::new(NumberSyntax::Tcl90) };
    }

    /// Install the numeric grammar for this thread's runtime. Call once during
    /// construction, before any script executes.
    pub fn set(syntax: NumberSyntax) {
        SYNTAX.with(|c| c.set(syntax));
    }

    /// The installed grammar (Tcl 9.0 until an embedder says otherwise).
    pub fn get() -> NumberSyntax {
        SYNTAX.with(Cell::get)
    }
}

/// Install the numeric grammar this runtime parses — see [`ambient`].
///
/// Call once while building the interpreter. Changing it mid-execution is not
/// supported: values already converted keep the numbers they were read as, so a
/// flip would leave the same script text meaning two different things.
pub fn set_runtime_syntax(syntax: NumberSyntax) {
    ambient::set(syntax);
}

/// The numeric grammar this runtime parses.
#[must_use]
pub fn runtime_syntax() -> NumberSyntax {
    ambient::get()
}

impl Default for ParseFlags {
    /// Every `TclParseNumber` bit clear, with the *runtime's* numeric grammar
    /// (see [`set_runtime_syntax`]) — so the existing call sites follow the
    /// dialect the runtime was built for without each threading it.
    fn default() -> Self {
        Self {
            integer_only: false,
            no_whitespace: false,
            no_underscore: false,
            syntax: ambient::get(),
        }
    }
}

impl ParseFlags {
    /// Flags carrying `syntax` with every `TclParseNumber` bit clear.
    #[must_use]
    pub fn for_syntax(syntax: NumberSyntax) -> Self {
        Self {
            syntax,
            ..Self::default()
        }
    }

    /// Whether `_` digit separators are accepted: both the caller's flag bit
    /// and the release's grammar have to allow them (they are 9.0+).
    fn underscores_ok(self) -> bool {
        !self.no_underscore && self.syntax.allows_digit_separators()
    }
}

/// **Which release's numeral grammar a consumer reads under.** The one place to
/// say that, so no caller re-assembles it from `ParseFlags` and a default.
///
/// Tcl's numeral grammar is release-dependent (see [`NumberSyntax`]), and there
/// are exactly three ways a consumer can stand in relation to it:
///
/// | variant | who | why |
/// |---|---|---|
/// | [`Numbers::Target`] | the compiler, codegen, the analyser, the LSP | compiling *for* one release, which it knows: `Module.dialect`, the registry's profile, the document's dialect |
/// | [`Numbers::Runtime`] | an interpreter evaluating a script | built for one release and does not switch, exactly as C settles it with a build-time `#define`; the ambient is installed once by `set_runtime_syntax` |
/// | [`Numbers::Unknown`] | a `ConstFoldFn`, an optimiser gate, a registry shape predicate | no release reachable, so it answers only what *every* release agrees on |
///
/// The third is the subtle one. Answering under one release's grammar when you
/// do not know the target bakes that release's reading into a program built for
/// another — so `Unknown` abstains instead, and each query below documents which
/// way its abstention points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numbers {
    /// A named release — a compile target, or an interpreter's own release.
    Target(NumberSyntax),
    /// Whatever grammar this runtime was built for (see [`set_runtime_syntax`]).
    Runtime,
    /// No release in hand; answer only what every release agrees on.
    Unknown,
}

impl Numbers {
    /// The grammar of `profile`, or the 9.x default when none is loaded.
    #[must_use]
    pub fn of_profile(profile: Option<&tcl_dialect::DialectProfile>) -> Self {
        Self::Target(NumberSyntax::of_profile(profile))
    }

    /// The grammar of the dialect `name` resolves to.
    #[must_use]
    pub fn of_dialect_name(name: Option<&str>) -> Self {
        Self::Target(NumberSyntax::of_dialect_name(name))
    }

    /// The single grammar this reads under, if there is one. `None` for
    /// [`Numbers::Unknown`], whose queries span every grammar instead.
    #[must_use]
    pub fn syntax(self) -> Option<NumberSyntax> {
        match self {
            Self::Target(n) => Some(n),
            Self::Runtime => Some(runtime_syntax()),
            Self::Unknown => None,
        }
    }

    /// Answer `f` under this release — or, when unknown, only if every release
    /// agrees.
    #[must_use]
    pub fn resolve<T: PartialEq>(self, f: impl Fn(NumberSyntax) -> T) -> Option<T> {
        match self.syntax() {
            Some(n) => Some(f(n)),
            None => NumberSyntax::unanimous(f),
        }
    }

    /// [`ParseFlags`] for this release; `integer_only` when asked.
    #[must_use]
    pub fn flags(self, integer_only: bool) -> Option<ParseFlags> {
        self.syntax().map(|n| ParseFlags {
            integer_only,
            ..ParseFlags::for_syntax(n)
        })
    }

    /// Parse `s` as a complete number. When unknown, yields a value only if
    /// every release reads the same one — so a release-dependent spelling is
    /// [`None`] rather than a guess.
    #[must_use]
    pub fn parse_whole(self, s: &str) -> Option<Number> {
        self.resolve(|n| parse_whole_with(s, ParseFlags::for_syntax(n)))
            .flatten()
    }

    /// Whether `s` is *entirely* a number. When unknown, requires every release
    /// to agree that it is — the sound reading for a claim that a value **is** a
    /// number (see [`Self::is_number_in_any_release`] for the opposite gate).
    #[must_use]
    pub fn is_whole_number(self, s: &str) -> bool {
        self.resolve(|n| is_whole_number(s, n)).unwrap_or(false)
    }

    /// Whether *any* release reads `s` as a number — the permissive direction,
    /// for a gate that refuses an optimisation because a value could still be
    /// numeric. Identical to [`Self::is_whole_number`] for a named release.
    #[must_use]
    pub fn is_number_in_any_release(self, s: &str) -> bool {
        match self.syntax() {
            Some(n) => is_whole_number(s, n),
            None => NumberSyntax::any(|n| is_whole_number(s, n)),
        }
    }

    /// Parse `s` as a whole integer, rejecting floats, NaN and past-wide
    /// magnitudes. Abstains like [`Self::parse_whole`] when unknown.
    #[must_use]
    pub fn parse_wide(self, s: &str) -> Option<i64> {
        self.resolve(|n| {
            match parse_whole_with(
                s,
                ParseFlags {
                    integer_only: true,
                    ..ParseFlags::for_syntax(n)
                },
            ) {
                Some(Number::Int(v)) => Some(v),
                _ => None,
            }
        })
        .flatten()
    }
}

/// A successful parse: the classified [`Number`] and the byte offset just past
/// the consumed number (the `endPtr` of `TclParseNumber`).
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// The classified value.
    pub number: Number,
    /// Offset in the input just past the number (before any trailing space).
    pub end: usize,
}

/// Format an `f64` as Tcl's canonical double string, byte-for-byte with
/// `Tcl_PrintDouble` (`tclStrToD.c`): `NaN`, `Inf`/`-Inf`, signed zero
/// (`0.0`/`-0.0`), and otherwise the shortest decimal that round-trips, laid out
/// `%g`-style — fixed notation when the decimal point sits in `[-3, 17]`, else
/// scientific (`d.ddde±NN`). An integer-valued result in fixed notation keeps a
/// `.0` suffix so it re-parses as a `Double`, not an `Int` (`2.0` not `2`,
/// `1e16` as `10000000000000000.0` not `10000000000000000`). The one shared
/// number→string formatter for the runtime's `double` rep and the compiler's
/// const-folder.
///
/// We must NOT use Rust's bare `{}` here: for large/small magnitudes it picks a
/// different fixed/scientific cutover than C Tcl (`{}` prints `1e300` as 301
/// digits and `1e17` as `100000000000000000`, the latter re-parsing as an
/// integer), so the `.0`-vs-Int round-trip and the canonical text both break.
/// Instead we take Rust's shortest scientific form (`{:e}`, which selects the
/// same minimal digits as C's `dtoa`) and re-lay it out under Tcl's rule.
#[must_use]
pub fn format_double(f: f64) -> String {
    if f.is_nan() {
        return "NaN".to_owned();
    }
    if f.is_infinite() {
        return if f.is_sign_negative() {
            "-Inf".to_owned()
        } else {
            "Inf".to_owned()
        };
    }
    // Signed zero must keep its sign: `Tcl_PrintDouble` renders `-0.0` as `-0.0`,
    // and `f == 0.0` is true for both zeros (so the sign bit is the only tell).
    if f == 0.0 {
        return if f.is_sign_negative() {
            "-0.0".to_owned()
        } else {
            "0.0".to_owned()
        };
    }

    // `{:e}` gives the shortest round-tripping mantissa + a base-10 exponent,
    // e.g. `1.5e2`, `1e17`, `6.022e23`. Split them to drive Tcl's layout.
    let neg = f < 0.0;
    let sci = format!("{:e}", f.abs());
    let (mantissa, exp_str) = sci
        .split_once('e')
        .expect("Rust's {:e} always emits an 'e' separator");
    let exp: i32 = exp_str
        .parse()
        .expect("Rust's {:e} exponent is a valid integer");
    // `digits` are the significant digits (point removed); `decpt` is the
    // position of the decimal point measured from the first digit, so the value
    // is `0.<digits> * 10^decpt` (matching `dtoa`'s `decpt`).
    let digits: String = mantissa.chars().filter(|&c| c != '.').collect();
    let ndigits = digits.len();
    let decpt = exp + 1;

    // Tcl's `%g` cutover: scientific outside `decpt ∈ [-3, 17]`, else fixed.
    let out = if !(-3..=17).contains(&decpt) {
        // Scientific: the shortest mantissa verbatim (a single digit stays bare,
        // no `.0`), then `e`, an explicit sign, and the natural-width exponent.
        let e = decpt - 1;
        format!("{mantissa}e{}{}", if e < 0 { '-' } else { '+' }, e.abs())
    } else if decpt <= 0 {
        // Leading-zero fraction: `0.000<digits>` (`-decpt` is ≥ 0 here).
        let lead = decpt.unsigned_abs() as usize;
        let zeros = "0".repeat(lead);
        format!("0.{zeros}{digits}")
    } else {
        // `decpt > 0` here, so it converts to a length cleanly.
        let point = usize::try_from(decpt).unwrap_or(usize::MAX);
        if point >= ndigits {
            // Integer-valued in fixed notation: pad to the point, then the `.0`
            // that keeps it parsing as a Double rather than an Int.
            let zeros = "0".repeat(point - ndigits);
            format!("{digits}{zeros}.0")
        } else {
            // A point inside the digits: `<int>.<frac>` (`0 < point < ndigits`).
            let (int_part, frac_part) = digits.split_at(point);
            format!("{int_part}.{frac_part}")
        }
    };
    if neg { format!("-{out}") } else { out }
}

/// Exactly compare an integer against a double, the way C Tcl's
/// `TclCompareTwoNumbers` (`tclExecute.c`) does for its wide/double arm —
/// **without** first rounding the integer to a double, which above 2⁵³ merges
/// distinct values (`expr {9007199254740993 > 9007199254740992.0}` is 1; a
/// both-as-`f64` comparison calls them equal). `None` means the double is NaN
/// (unordered — C Tcl's rule for a NaN comparison operand is "`!=` is true,
/// every other comparison false", which the caller applies).
///
/// Takes `i128` so the runtime's `Big` tier shares it; `i64` callers widen.
#[must_use]
pub fn compare_int_double(w: i128, d: f64) -> Option<core::cmp::Ordering> {
    use core::cmp::Ordering;
    // 2¹²⁷ — the smallest double at or above every i128 (`i128::MAX` = 2¹²⁷−1).
    // At or beyond it (incl. +Inf) every integer is smaller; strictly below
    // −2¹²⁷ (incl. −Inf) every integer is greater. −2¹²⁷ itself is exactly
    // `i128::MIN`, so it falls through to the exact path.
    const TWO_POW_127: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;
    if d.is_nan() {
        return None;
    }
    if d >= TWO_POW_127 {
        return Some(Ordering::Less);
    }
    if d < -TWO_POW_127 {
        return Some(Ordering::Greater);
    }
    // Finite with |d| ≤ 2¹²⁷: the integer part is exactly representable, so
    // compare integer parts first and let a non-zero fraction break the tie.
    let (int_part, has_fraction) = split_double_exact(d);
    Some(match w.cmp(&int_part) {
        Ordering::Equal if !has_fraction => Ordering::Equal,
        // Same integer part but `d` carries a fraction: truncation is toward
        // zero, so a positive `d` sits above its integer part (w < d) and a
        // negative `d` below it (w > d).
        Ordering::Equal if d > 0.0 => Ordering::Less,
        Ordering::Equal => Ordering::Greater,
        unequal => unequal,
    })
}

/// Split a finite double with |d| ≤ 2¹²⁷ into its exact truncated integer part
/// and whether a non-zero fractional part was cut off. Pure bit arithmetic on
/// the IEEE-754 representation — a `d as i128` cast would truncate silently on
/// out-of-range input and trip `clippy::cast_possible_truncation`; this stays
/// exact by construction.
fn split_double_exact(d: f64) -> (i128, bool) {
    const MANTISSA_BITS: u64 = 52;
    const MANTISSA_MASK: u64 = (1 << MANTISSA_BITS) - 1;
    const EXPONENT_MASK: u64 = 0x7ff;
    // IEEE-754 binary64 bias for the stored exponent, plus the 52 mantissa
    // fraction bits: value = mantissa × 2^(stored − 1023 − 52).
    const EXPONENT_BIAS_AND_SHIFT: u64 = 1023 + MANTISSA_BITS;

    let bits = d.to_bits();
    let negative = (bits >> 63) == 1;
    let stored_exponent = (bits >> MANTISSA_BITS) & EXPONENT_MASK;
    let fraction_bits = bits & MANTISSA_MASK;
    if stored_exponent == 0 {
        // Zero (fraction 0) or subnormal (|d| < 2⁻¹⁰²²): integer part is 0
        // either way; only a subnormal leaves a fraction behind.
        return (0, fraction_bits != 0);
    }
    // Normal number: an implicit leading 1 above the 52 stored fraction bits.
    let mantissa = (1 << MANTISSA_BITS) | fraction_bits;
    let (magnitude, has_fraction) =
        if let Some(left) = stored_exponent.checked_sub(EXPONENT_BIAS_AND_SHIFT) {
            // 2^left scales the mantissa up: purely integral. The caller's range
            // bound (|d| ≤ 2¹²⁷) keeps the result within a u128; saturate rather
            // than panic if the bound is ever violated.
            let scaled = u32::try_from(left)
                .ok()
                .and_then(|shift| u128::from(mantissa).checked_shl(shift))
                .unwrap_or(u128::MAX);
            (scaled, false)
        } else {
            // Scaling down by 2^right: the low `right` bits are the fraction.
            let right = EXPONENT_BIAS_AND_SHIFT - stored_exponent;
            if right > MANTISSA_BITS {
                // The whole mantissa is fractional (|d| < 1).
                (0, true)
            } else {
                (
                    u128::from(mantissa >> right),
                    mantissa & ((1 << right) - 1) != 0,
                )
            }
        };
    // Magnitude ≤ 2¹²⁷ by the caller's bound. Only −2¹²⁷ (`i128::MIN`) uses the
    // top bit, and only on the negative side; every other value converts.
    let int_part = if negative {
        i128::try_from(magnitude).map_or(i128::MIN, |m| -m)
    } else {
        i128::try_from(magnitude).unwrap_or(i128::MAX)
    };
    (int_part, has_fraction)
}

/// Tcl numeric whitespace: space, tab, newline, VT, FF, CR.
#[inline]
fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

#[inline]
fn digit_val(c: u8, radix: u32) -> Option<u32> {
    let v = match c {
        b'0'..=b'9' => u32::from(c - b'0'),
        b'a'..=b'f' => u32::from(c - b'a') + 10,
        b'A'..=b'F' => u32::from(c - b'A') + 10,
        _ => return None,
    };
    (v < radix).then_some(v)
}

/// Whether `text` is *entirely* a number under `syntax` — the test C's
/// `ParseLexeme` makes by calling `TclParseNumber` and requiring it to consume
/// the whole lexeme before classifying the text as `NUMBER`. A numeral whose
/// digits are invalid for its radix (`0o8`), whose prefix does not exist in this
/// release (`0d99` before 9.0), or which is a bare prefix (`0x`) fails here and
/// is a bareword instead.
#[must_use]
pub fn is_whole_number(text: &str, syntax: NumberSyntax) -> bool {
    parse_whole_with(text, ParseFlags::for_syntax(syntax)).is_some()
}

/// Whether `text` is one complete expression-number lexeme under this release.
///
/// This is the expression-specific companion to [`is_whole_number`]. It first
/// asks the lower shared boundary owner (`tcl_dialect::scan_expr_number`) to
/// prove that the token ends where C `ParseLexeme` ends, then uses this module's
/// value parser to classify it. The split is intentional: the dialect crate
/// owns byte boundaries below both the lexer and syntax parser, while this
/// module remains the sole `TclParseNumber` value implementation.
#[must_use]
pub fn is_expr_number(
    text: &str,
    syntax: NumberSyntax,
    expr_grammar_base: Option<tcl_dialect::TclVersion>,
) -> bool {
    tcl_dialect::scan_expr_number(text.as_bytes(), 0, syntax, expr_grammar_base)
        .is_some_and(|lexeme| lexeme.end() == text.len())
        && is_whole_number(text, syntax)
}

/// Parse a number at the start of `s` (after optional leading whitespace),
/// returning the classified value and where it ended. Returns `None` if no valid
/// number begins there. This is the partial form (the lexer/`scan` entry); use
/// [`parse_whole`] when the entire string must be a number.
#[must_use]
pub fn parse(s: &str, flags: ParseFlags) -> Option<Parsed> {
    let b = s.as_bytes();
    let len = b.len();
    let mut i = 0;

    if !flags.no_whitespace {
        while i < len && is_ws(b[i]) {
            i += 1;
        }
    }

    // Optional sign.
    let mut negative = false;
    if i < len && (b[i] == b'+' || b[i] == b'-') {
        negative = b[i] == b'-';
        i += 1;
    }

    // Inf / NaN (alphabetic forms).
    if i < len && matches!(b[i], b'i' | b'I' | b'n' | b'N') {
        return parse_inf_nan(b, i, negative, flags.syntax);
    }

    // Radix prefix, each requiring a following digit. `NumberSyntax` owns the
    // release table (`0x` universally, `0b`/`0o` from 8.5, `0d` from 9.0), so
    // this value parser and the lower expression-boundary scanner cannot drift.
    // An unavailable prefix simply is not a prefix — `0o17` under 8.4 parses
    // as the single digit `0` with `o17` left over, which makes it a bareword
    // in an expression.
    let mut radix = Radix::Dec;
    let mut int_only_prefix = flags.integer_only;
    if i + 1 < len && b[i] == b'0' {
        let r = flags
            .syntax
            .explicit_radix(b[i + 1])
            .map(|base| match base {
                2 => Radix::Bin,
                8 => Radix::Oct,
                10 => Radix::Dec,
                16 => Radix::Hex,
                _ => unreachable!("NumberSyntax only exposes Tcl's four explicit radices"),
            });
        if let Some(r) = r {
            // Commit to the prefix only if a valid digit follows it.
            if i + 2 < len && digit_val(b[i + 2], r as u32).is_some() {
                radix = r;
                int_only_prefix = true; // `0x`/`0o`/`0b`/`0d` are integer forms
                i += 2;
            }
        }
    }

    // Octal-by-leading-zero, up to 8.6 (`#undef KILL_OCTAL`): a bare `0`
    // followed by digits is octal, so `0755` is 493 and `08` yields `0` with
    // `8` left over — C's octal scan simply stops at the invalid digit. Only
    // for an *integer*: a digit run followed by `.`/`e` is a decimal float in
    // every release (`0.5`, `07.5`), which C reaches by backtracking out of its
    // octal state.
    if radix == Radix::Dec
        && flags.syntax.leading_zero_is_octal()
        // `get`, not `b[i]`: the input can be exhausted here (an empty string, a
        // whitespace-only one, or a lone sign), and under a pre-9.0 syntax this
        // is the first byte read — an index would panic instead of reporting
        // "not a number". Reachable from every runtime call site once the
        // ambient grammar is 8.x (`expr {$empty + 1}`).
        && b.get(i) == Some(&b'0')
        && i + 1 < len
        && b[i + 1].is_ascii_digit()
        && !decimal_run_is_fractional(b, i)
    {
        radix = Radix::Oct;
        int_only_prefix = true;
    }

    // Scan the integer magnitude digits (with `_` separators).
    let int_start = i;
    let scan = scan_digits(b, i, radix as u32, !flags.underscores_ok());
    i = scan.end;

    // Decimal floats: a `.` or exponent (and not integer-only) makes it a double.
    // Also a leading `.` with no integer digits (`.5`).
    let has_int_digits = i > int_start;
    let dot = i < len && b[i] == b'.';
    let exp = i < len && (b[i] == b'e' || b[i] == b'E');
    if radix == Radix::Dec && !int_only_prefix && (dot || exp || !has_int_digits) {
        return parse_decimal_float(b, int_start, negative, flags);
    }
    if !has_int_digits {
        return None; // no digits and not a float → not a number
    }

    // Integer: a wide `Int` when the magnitude fits, else `Big` from the cleaned
    // digits (the wide `u64` accumulator can still exceed `i64`'s range without
    // overflowing — e.g. magnitudes in `(i64::MAX, u64::MAX]`).
    let number = match (scan.overflow, to_i64(scan.magnitude, negative)) {
        (false, Some(v)) => Number::Int(v),
        _ => Number::Big {
            negative,
            radix,
            digits: scan.digits.into_owned(),
        },
    };
    Some(Parsed { number, end: i })
}

/// Whether the decimal-digit run at `i` is followed by `.` or an exponent — in
/// which case it is a decimal float, not an octal integer, in every release.
/// The lookahead spans decimal digits (not octal ones) so `08.5` and `09e1` are
/// recognised as floats just as C's backtracking does.
fn decimal_run_is_fractional(b: &[u8], i: usize) -> bool {
    let mut j = i;
    while j < b.len() && b[j].is_ascii_digit() {
        j += 1;
    }
    j < b.len() && matches!(b[j], b'.' | b'e' | b'E')
}

/// Apply `negative` to a `u64` magnitude, yielding `Some(i64)` when it fits a
/// wide (`i64::MIN`'s magnitude is `2^63`, representable only when negated).
fn to_i64(mag: u64, negative: bool) -> Option<i64> {
    if negative {
        if mag == 1u64 << 63 {
            Some(i64::MIN)
        } else {
            i64::try_from(mag).ok().map(|v| -v)
        }
    } else {
        i64::try_from(mag).ok()
    }
}

/// Parse `s` as a complete Tcl number: optional surrounding whitespace, then the
/// whole remainder must be the number (the `Tcl_GetWideIntFromObj`/`…Double…`
/// shape). Returns `None` on trailing junk.
#[must_use]
pub fn parse_whole(s: &str) -> Option<Number> {
    parse_whole_with(s, ParseFlags::default())
}

/// [`parse_whole`] with explicit [`ParseFlags`] — e.g. `integer_only` for the
/// whole-string integer shape (`Tcl_GetWideIntFromObj` via
/// `TCL_PARSE_INTEGER_ONLY`), where a fractional part is trailing junk.
/// Trailing whitespace is skipped unless `no_whitespace` forbids it.
#[must_use]
pub fn parse_whole_with(s: &str, flags: ParseFlags) -> Option<Number> {
    let p = parse(s, flags)?;
    let mut i = p.end;
    let b = s.as_bytes();
    while !flags.no_whitespace && i < b.len() && is_ws(b[i]) {
        i += 1;
    }
    (i == b.len()).then_some(p.number)
}

struct IntScan<'s> {
    end: usize,
    /// Accumulated magnitude (valid only when `!overflow`).
    magnitude: u64,
    /// The wide magnitude overflowed `u64` — use `digits` for a bignum.
    overflow: bool,
    /// Magnitude digits with `_` separators removed (borrowed when none).
    digits: Cow<'s, str>,
}

/// Scan a run of base-`radix` digits with optional `_` separators (only between
/// two digits). Accumulates the magnitude into `u64`, flagging overflow.
fn scan_digits(b: &[u8], start: usize, radix: u32, no_underscore: bool) -> IntScan<'_> {
    let len = b.len();
    let mut i = start;
    let mut mag: u64 = 0;
    let mut overflow = false;
    let mut saw_underscore = false;
    while i < len {
        if let Some(d) = digit_val(b[i], radix) {
            if !overflow {
                match mag
                    .checked_mul(u64::from(radix))
                    .and_then(|m| m.checked_add(u64::from(d)))
                {
                    Some(m) => mag = m,
                    None => overflow = true,
                }
            }
            i += 1;
        } else if !no_underscore
            && b[i] == b'_'
            && i > start
            && digit_val(b[i - 1], radix).is_some()
        {
            let mut j = i + 1;
            while j < len && b[j] == b'_' {
                j += 1;
            }
            if j < len && digit_val(b[j], radix).is_some() {
                saw_underscore = true;
                i = j;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    // Cleaned digits (for the bignum path): strip `_`. Borrow when there were none.
    let raw = std::str::from_utf8(&b[start..i]).unwrap_or("");
    let digits = if saw_underscore {
        Cow::Owned(raw.replace('_', ""))
    } else {
        Cow::Borrowed(raw)
    };
    IntScan {
        end: i,
        magnitude: mag,
        overflow,
        digits,
    }
}

/// Parse a decimal float lexeme starting at `start` (after the sign), via Rust's
/// correctly-rounded `f64` parser once `_` separators are stripped.
fn parse_decimal_float(
    b: &[u8],
    start: usize,
    negative: bool,
    flags: ParseFlags,
) -> Option<Parsed> {
    let len = b.len();
    let mut i = start;
    let mut any_digit = false;

    let scan_run = |b: &[u8], mut i: usize| -> usize {
        while i < len {
            if b[i].is_ascii_digit() {
                i += 1;
            } else if flags.underscores_ok()
                && b[i] == b'_'
                && i > start
                && b[i - 1].is_ascii_digit()
            {
                let mut j = i + 1;
                while j < len && b[j] == b'_' {
                    j += 1;
                }
                if j < len && b[j].is_ascii_digit() {
                    i = j;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        i
    };

    let after_int = scan_run(b, i);
    if after_int > i {
        any_digit = true;
    }
    i = after_int;
    if i < len && b[i] == b'.' {
        i += 1;
        let after_frac = scan_run(b, i);
        if after_frac > i {
            any_digit = true;
        }
        i = after_frac;
    }
    if !any_digit {
        return None;
    }
    if i < len && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < len && (b[j] == b'+' || b[j] == b'-') {
            j += 1;
        }
        let after_exp = scan_run(b, j);
        if after_exp > j {
            i = after_exp; // a valid exponent — consume it
        }
        // else: the `e` is not part of the number; stop before it.
    }

    let lexeme = std::str::from_utf8(&b[start..i]).ok()?;
    let cleaned = if lexeme.contains('_') {
        Cow::Owned(lexeme.replace('_', ""))
    } else {
        Cow::Borrowed(lexeme)
    };
    let mag: f64 = cleaned.parse().ok()?;
    let value = if negative { -mag } else { mag };
    Some(Parsed {
        number: Number::Double(value),
        end: i,
    })
}

/// Parse the alphabetic `Inf`/`Infinity`/`NaN`/`NaN(hex)` forms (case-insensitive).
fn parse_inf_nan(b: &[u8], start: usize, negative: bool, syntax: NumberSyntax) -> Option<Parsed> {
    let len = b.len();
    // Case-insensitive prefix match; returns the end offset if matched.
    let matches_ci = |word: &[u8], at: usize| -> bool {
        at + word.len() <= len && b[at..at + word.len()].eq_ignore_ascii_case(word)
    };

    if matches_ci(b"inf", start) {
        let mut end = start + 3;
        if matches_ci(b"inity", end) {
            end += 5; // the full "Infinity"
        }
        let v = if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
        return Some(Parsed {
            number: Number::Double(v),
            end,
        });
    }

    if matches_ci(b"nan", start) {
        let mut end = start + 3;
        let mut payload = None;
        // 8.5+ shares the lower C `TclParseNumber` payload state machine with
        // the expression lexer: one through thirteen hex digits, ASCII
        // whitespace ignored, and a fourteenth digit invalidates the whole
        // parenthesised form. Tcl 8.4 delegates special floats to `strtod`,
        // whose `GetLexeme` path has no parenthesised payload grammar.
        if syntax != NumberSyntax::Tcl84
            && let Some(lexeme) = tcl_dialect::scan_nan_payload(b, end)
        {
            payload = Some(lexeme.value());
            end = lexeme.end();
        }
        return Some(Parsed {
            number: Number::Nan { negative, payload },
            end,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(s: &str) -> Option<Number> {
        parse_whole(s)
    }

    #[test]
    fn decimal_integers() {
        assert_eq!(n("0"), Some(Number::Int(0)));
        assert_eq!(n("42"), Some(Number::Int(42)));
        assert_eq!(n("-42"), Some(Number::Int(-42)));
        assert_eq!(n("+7"), Some(Number::Int(7)));
        // leading zero is DECIMAL in 9.0, not octal
        assert_eq!(n("0755"), Some(Number::Int(755)));
        assert_eq!(n("  13  "), Some(Number::Int(13))); // surrounding whitespace ok
    }

    #[test]
    fn radix_prefixes() {
        assert_eq!(n("0xff"), Some(Number::Int(255)));
        assert_eq!(n("0XFF"), Some(Number::Int(255)));
        assert_eq!(n("0o17"), Some(Number::Int(15)));
        assert_eq!(n("0b1010"), Some(Number::Int(10)));
        assert_eq!(n("0d0755"), Some(Number::Int(755)));
        assert_eq!(n("-0x10"), Some(Number::Int(-16)));
        // a bare `0x` (no hex digit) is not a whole number
        assert_eq!(n("0x"), None);
    }

    #[test]
    fn underscores() {
        assert_eq!(n("1_000_000"), Some(Number::Int(1_000_000)));
        assert_eq!(n("1__000___000"), Some(Number::Int(1_000_000)));
        assert_eq!(n("0xff_ff"), Some(Number::Int(0xffff)));
        assert_eq!(n("0xff__ff"), Some(Number::Int(0xffff)));
        assert_eq!(n("0b1__0"), Some(Number::Int(2)));
        assert_eq!(n("0o7__7"), Some(Number::Int(63)));
        assert_eq!(n("1_0"), Some(Number::Int(10)));
        // misplaced underscores → not a whole number
        assert_eq!(n("_1"), None);
        assert_eq!(n("1_"), None);
        assert_eq!(n("1__"), None);
        assert_eq!(n("0x_f"), None);
    }

    #[test]
    fn i64_bounds_and_bignum() {
        assert_eq!(n("9223372036854775807"), Some(Number::Int(i64::MAX)));
        assert_eq!(n("-9223372036854775808"), Some(Number::Int(i64::MIN)));
        // one past i64::MAX → Big
        assert_eq!(
            n("9223372036854775808"),
            Some(Number::Big {
                negative: false,
                radix: Radix::Dec,
                digits: "9223372036854775808".to_owned()
            })
        );
        // a clearly huge value → Big
        assert_eq!(
            n("123456789012345678901234567890"),
            Some(Number::Big {
                negative: false,
                radix: Radix::Dec,
                digits: "123456789012345678901234567890".to_owned()
            })
        );
        // huge hex with separators → Big with cleaned digits
        assert_eq!(
            n("0xffff_ffff_ffff_ffff_f"),
            Some(Number::Big {
                negative: false,
                radix: Radix::Hex,
                digits: "fffffffffffffffff".to_owned()
            })
        );
    }

    #[test]
    fn floats() {
        assert_eq!(n("1.5"), Some(Number::Double(1.5)));
        assert_eq!(n(".5"), Some(Number::Double(0.5)));
        assert_eq!(n("1."), Some(Number::Double(1.0)));
        assert_eq!(n("-2.5e3"), Some(Number::Double(-2500.0)));
        assert_eq!(n("1E-3"), Some(Number::Double(0.001)));
        assert_eq!(n("1_0.5"), Some(Number::Double(10.5)));
        assert_eq!(n("1__0.5"), Some(Number::Double(10.5)));
        assert_eq!(n("1.0__2"), Some(Number::Double(1.02)));
        assert_eq!(n("1_.0"), None);
        assert_eq!(n("1._0"), None);
    }

    #[test]
    fn inf_and_nan() {
        assert_eq!(n("Inf"), Some(Number::Double(f64::INFINITY)));
        assert_eq!(n("infinity"), Some(Number::Double(f64::INFINITY)));
        assert_eq!(n("-Inf"), Some(Number::Double(f64::NEG_INFINITY)));
        assert_eq!(
            n("NaN"),
            Some(Number::Nan {
                negative: false,
                payload: None
            })
        );
        assert_eq!(
            n("NaN(1ff)"),
            Some(Number::Nan {
                negative: false,
                payload: Some(0x1ff)
            })
        );
        assert_eq!(
            n("NaN( 1 2 3 )"),
            Some(Number::Nan {
                negative: false,
                payload: Some(0x123)
            })
        );
        assert_eq!(n("NaN(123456789abcde)"), None);
    }

    #[test]
    fn integer_only_flag() {
        // integer_only stops at the `.` so a whole-parse of a float fails
        let f = ParseFlags {
            integer_only: true,
            ..Default::default()
        };
        let p = parse("12.5", f).unwrap();
        assert_eq!(p.number, Number::Int(12));
        assert_eq!(p.end, 2); // stopped before the `.`
    }

    #[test]
    fn not_numbers() {
        assert_eq!(n(""), None);
        assert_eq!(n("   "), None);
        assert_eq!(n("abc"), None);
        assert_eq!(n("12x"), None);
        assert_eq!(n("0x1.5"), None); // no hex floats
        assert_eq!(n("1.2.3"), None);
    }

    #[test]
    fn partial_parse_reports_end() {
        let p = parse("42 rest", ParseFlags::default()).unwrap();
        assert_eq!(p.number, Number::Int(42));
        assert_eq!(p.end, 2);
    }

    #[test]
    fn format_double_matches_tcl_print_double() {
        // Specials and signed zero (the `-0.0` sign must survive — regression).
        assert_eq!(format_double(f64::NAN), "NaN");
        assert_eq!(format_double(f64::INFINITY), "Inf");
        assert_eq!(format_double(f64::NEG_INFINITY), "-Inf");
        assert_eq!(format_double(0.0), "0.0");
        assert_eq!(format_double(-0.0), "-0.0");
        // Ordinary fixed-notation values keep the distinguishing `.0`.
        assert_eq!(format_double(2.0), "2.0");
        assert_eq!(format_double(1.5), "1.5");
        assert_eq!(format_double(0.5), "0.5");
        assert_eq!(format_double(0.0001), "0.0001");
        // Integer-valued doubles ≥ 1e16 keep `.0` (re-parse as Double, not Int):
        // tclsh renders these with the trailing `.0`, not as a bare integer.
        assert_eq!(format_double(1e16), "10000000000000000.0");
        assert_eq!(format_double(1.5e16), "15000000000000000.0");
        assert_eq!(format_double(-1e16), "-10000000000000000.0");
        // At 1e17 tclsh switches to scientific (`1e+17`, single-digit mantissa
        // stays bare, exponent has an explicit sign and natural width).
        assert_eq!(format_double(1e17), "1e+17");
        assert_eq!(format_double(-1e17), "-1e+17");
        assert_eq!(format_double(1e-5), "1e-5");
        assert_eq!(format_double(6.022e23), "6.022e+23");
    }

    #[test]
    fn format_double_round_trips_to_double_not_int() {
        // The point of the `.0`: every `format_double` output must re-parse as a
        // `Double` (never an `Int`), so the numeric tower stays put across a
        // string round-trip. 1e16/1e17 are the regression cases.
        for &v in &[2.0, 1e16, 1.5e16, 1e17, -1e16, -1e17, 1e-5, 1.0, 0.0, -0.0] {
            let s = format_double(v);
            match parse_whole(&s) {
                Some(Number::Double(_)) => {}
                other => panic!("format_double({v}) = {s:?} re-parsed as {other:?}, not Double"),
            }
        }
    }

    #[test]
    fn compare_int_double_is_exact_past_2_pow_53() {
        use core::cmp::Ordering::{Equal, Greater, Less};
        // The motivating cases (tclsh oracle): a both-as-f64 comparison calls
        // 9007199254740993 equal to 9007199254740992.0; Tcl compares exactly.
        //   expr {9007199254740993 == 9007199254740992.0} → 0
        //   expr {9007199254740993 >  9007199254740992.0} → 1
        assert_eq!(
            compare_int_double(9_007_199_254_740_993, 9_007_199_254_740_992.0),
            Some(Greater)
        );
        // The float literal ...993.0 itself rounds to ...992.0, so the wide
        // still orders above it: expr {9007199254740993 > 9007199254740993.0} → 1.
        assert_eq!(
            compare_int_double(9_007_199_254_740_993, 9_007_199_254_740_993.0),
            Some(Greater)
        );
        assert_eq!(
            compare_int_double(9_007_199_254_740_992, 9_007_199_254_740_992.0),
            Some(Equal)
        );
        // 20000000000000003 < 20000000000000004.0 — the exact case the C
        // comment in TclCompareTwoNumbers cites.
        assert_eq!(
            compare_int_double(20_000_000_000_000_003, 20_000_000_000_000_004.0),
            Some(Less)
        );
    }

    #[test]
    fn compare_int_double_small_and_fractional() {
        use core::cmp::Ordering::{Equal, Greater, Less};
        assert_eq!(compare_int_double(2, 2.5), Some(Less));
        assert_eq!(compare_int_double(3, 2.5), Some(Greater));
        assert_eq!(compare_int_double(-2, -2.5), Some(Greater));
        assert_eq!(compare_int_double(-3, -2.5), Some(Less));
        assert_eq!(compare_int_double(0, 0.0), Some(Equal));
        assert_eq!(compare_int_double(0, -0.0), Some(Equal));
        assert_eq!(compare_int_double(0, -0.5), Some(Greater));
        assert_eq!(compare_int_double(0, 0.5), Some(Less));
        assert_eq!(compare_int_double(5, 5.0), Some(Equal));
        assert_eq!(compare_int_double(-5, -5.0), Some(Equal));
        // Subnormals: integer part 0, fraction non-zero.
        assert_eq!(compare_int_double(0, f64::MIN_POSITIVE / 2.0), Some(Less));
        assert_eq!(
            compare_int_double(0, -f64::MIN_POSITIVE / 2.0),
            Some(Greater)
        );
    }

    #[test]
    fn compare_int_double_extremes() {
        use core::cmp::Ordering::{Equal, Greater, Less};
        // ±2¹²⁷: i128::MAX = 2¹²⁷−1 sits below 2¹²⁷; i128::MIN is exactly
        // −2¹²⁷. 2⁶³ marks the i64-boundary sliver.
        const TWO_POW_127: f64 = 170_141_183_460_469_231_731_687_303_715_884_105_728.0;
        const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
        // NaN is unordered — the caller applies Tcl's "!= true, rest false".
        assert_eq!(compare_int_double(1, f64::NAN), None);
        // Infinities order around every integer.
        assert_eq!(compare_int_double(i128::MAX, f64::INFINITY), Some(Less));
        assert_eq!(
            compare_int_double(i128::MIN, f64::NEG_INFINITY),
            Some(Greater)
        );
        assert_eq!(compare_int_double(i128::MAX, TWO_POW_127), Some(Less));
        assert_eq!(compare_int_double(i128::MIN, -TWO_POW_127), Some(Equal));
        assert_eq!(compare_int_double(0, -TWO_POW_127), Some(Greater));
        // 2⁶³ exactly vs the neighbouring wides (the i64-boundary sliver).
        assert_eq!(
            compare_int_double(i128::from(i64::MAX), TWO_POW_63),
            Some(Less)
        );
        assert_eq!(
            compare_int_double(i128::from(i64::MIN), -TWO_POW_63),
            Some(Equal)
        );
        // A wide whose f64 rounding lands ON 2⁶³ still orders exactly.
        assert_eq!(
            compare_int_double(9_223_372_036_854_775_807, 9.3e18),
            Some(Less)
        );
        assert_eq!(
            compare_int_double(-9_223_372_036_854_775_807, -9.3e18),
            Some(Greater)
        );
    }
}

#[cfg(test)]
mod dialect_tests {
    use super::{
        Number, NumberSyntax, Numbers, ParseFlags, is_expr_number, parse, parse_whole_with,
        runtime_syntax, set_runtime_syntax,
    };
    use tcl_dialect::TclVersion;

    fn whole(s: &str, syntax: NumberSyntax) -> Option<Number> {
        parse_whole_with(s, ParseFlags::for_syntax(syntax))
    }

    /// A bare leading zero is octal up to 8.6 and decimal from 9.0
    /// (`changes.md`: "`0NNN` format is no longer octal interpretation").
    #[test]
    fn leading_zero_is_octal_before_9_0() {
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert_eq!(whole("0755", syntax), Some(Number::Int(493)), "{syntax:?}");
            assert_eq!(whole("010", syntax), Some(Number::Int(8)), "{syntax:?}");
        }
        assert_eq!(whole("0755", NumberSyntax::Tcl90), Some(Number::Int(755)));
        assert_eq!(whole("010", NumberSyntax::Tcl90), Some(Number::Int(10)));
        // `0` alone, and a zero run, are the same in every release.
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            assert_eq!(whole("0", syntax), Some(Number::Int(0)), "{syntax:?}");
            assert_eq!(whole("00", syntax), Some(Number::Int(0)), "{syntax:?}");
        }
    }

    /// Under the octal rule an invalid octal digit stops the scan, so `08` is a
    /// *partial* parse (`0`, with `8` trailing) — `parse_whole` therefore
    /// rejects it, which is how C reports `08` as a bad octal number. From 9.0
    /// it is plain decimal 8.
    #[test]
    fn invalid_octal_digit_stops_the_scan_before_9_0() {
        for bad in ["08", "09"] {
            assert_eq!(whole(bad, NumberSyntax::Tcl85), None, "{bad}");
            let partial = parse(bad, ParseFlags::for_syntax(NumberSyntax::Tcl85)).unwrap();
            assert_eq!(partial.number, Number::Int(0));
            assert_eq!(partial.end, 1, "scan stops at the invalid octal digit");
        }
        assert_eq!(whole("08", NumberSyntax::Tcl90), Some(Number::Int(8)));
        assert_eq!(whole("09", NumberSyntax::Tcl90), Some(Number::Int(9)));
    }

    /// A digit run followed by `.` or an exponent is a decimal float in every
    /// release — C backtracks out of its octal state rather than reading `07.5`
    /// as octal.
    #[test]
    fn leading_zero_floats_stay_decimal() {
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            assert_eq!(
                whole("0.5", syntax),
                Some(Number::Double(0.5)),
                "{syntax:?}"
            );
            assert_eq!(
                whole("07.5", syntax),
                Some(Number::Double(7.5)),
                "{syntax:?}"
            );
            assert_eq!(
                whole("08.5", syntax),
                Some(Number::Double(8.5)),
                "{syntax:?}"
            );
            assert_eq!(
                whole("09e1", syntax),
                Some(Number::Double(90.0)),
                "{syntax:?}"
            );
        }
    }

    /// `0x` is universal; `0b`/`0o` arrive in 8.5; `0d` in 9.0. An unavailable
    /// prefix is not a prefix, so the whole-string parse fails (the trailing
    /// letters are junk) — which is what makes such a word a bareword in an
    /// expression.
    #[test]
    fn radix_prefixes_appear_in_the_release_that_added_them() {
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            assert_eq!(whole("0x1f", syntax), Some(Number::Int(31)), "{syntax:?}");
        }
        // 0b / 0o: absent in 8.4, present from 8.5.
        assert_eq!(whole("0o17", NumberSyntax::Tcl84), None);
        assert_eq!(whole("0b101", NumberSyntax::Tcl84), None);
        for syntax in [NumberSyntax::Tcl85, NumberSyntax::Tcl90] {
            assert_eq!(whole("0o17", syntax), Some(Number::Int(15)), "{syntax:?}");
            assert_eq!(whole("0b101", syntax), Some(Number::Int(5)), "{syntax:?}");
        }
        // 0d: 9.0 only.
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert_eq!(whole("0d99", syntax), None, "{syntax:?}");
        }
        assert_eq!(whole("0d99", NumberSyntax::Tcl90), Some(Number::Int(99)));
    }

    /// A radix-invalid digit never makes the whole string a number, in any
    /// release — the basis of C's `invalid bareword "0o8"` report.
    #[test]
    fn radix_invalid_digits_are_never_whole_numbers() {
        for syntax in [
            NumberSyntax::Tcl84,
            NumberSyntax::Tcl85,
            NumberSyntax::Tcl90,
        ] {
            for bad in ["0o8", "0o9", "0b2", "0b3", "0x", "0xg"] {
                assert_eq!(whole(bad, syntax), None, "{bad} under {syntax:?}");
            }
        }
    }

    /// `_` digit separators are 9.0+, and the `no_underscore` flag bit still
    /// overrides even there.
    #[test]
    fn digit_separators_are_9_0_only() {
        assert_eq!(
            whole("1__000", NumberSyntax::Tcl90),
            Some(Number::Int(1000))
        );
        assert_eq!(
            whole("0xff__ff", NumberSyntax::Tcl90),
            Some(Number::Int(65535))
        );
        for syntax in [NumberSyntax::Tcl84, NumberSyntax::Tcl85] {
            assert_eq!(whole("1__000", syntax), None, "{syntax:?}");
        }
        let no_sep = ParseFlags {
            no_underscore: true,
            syntax: NumberSyntax::Tcl90,
            ..ParseFlags::default()
        };
        assert_eq!(parse_whole_with("1__000", no_sep), None);
    }

    /// Expression classification consumes the lower shared boundary scanner
    /// before it asks this module's value parser. These paired rows fail if the
    /// lexer and syntax parser ever start using different junction rules.
    #[test]
    fn expression_number_validation_shares_the_boundary_owner() {
        assert!(is_expr_number(
            "1.0_2",
            NumberSyntax::Tcl90,
            Some(TclVersion::V9_0)
        ));
        assert!(!is_expr_number(
            "1.0_2",
            NumberSyntax::Tcl85,
            Some(TclVersion::V8_6)
        ));
        assert!(!is_expr_number(
            "1_eq",
            NumberSyntax::Tcl90,
            Some(TclVersion::V9_0)
        ));
        // The value parser consumes the hexadecimal prefix, while the shared
        // lower boundary scanner restarts at `ne`/`in`/`ge` rather than using
        // the preceding alpha hex digit as a suffix boundary.
        for (source, syntax, base, end) in [
            ("0xfne 1", NumberSyntax::Tcl85, Some(TclVersion::V8_6), 3),
            (
                "0xffin {255}",
                NumberSyntax::Tcl85,
                Some(TclVersion::V8_6),
                4,
            ),
            ("0xfge 15", NumberSyntax::Tcl90, Some(TclVersion::V9_0), 3),
        ] {
            assert_eq!(
                tcl_dialect::scan_expr_number(source.as_bytes(), 0, syntax, base,)
                    .unwrap()
                    .end(),
                end,
                "{source}"
            );
            assert_eq!(
                parse(source, ParseFlags::for_syntax(syntax)).unwrap().end,
                end,
                "{source}"
            );
        }
        assert!(is_expr_number(
            "NaN(1)",
            NumberSyntax::Tcl85,
            Some(TclVersion::V8_6)
        ));
        assert!(is_expr_number(
            "NaN( 1 2 3 )",
            NumberSyntax::Tcl90,
            Some(TclVersion::V9_0)
        ));
        assert!(!is_expr_number(
            "NaN(123456789abcde)",
            NumberSyntax::Tcl90,
            Some(TclVersion::V9_0)
        ));
        assert!(!is_expr_number(
            "NaN(1)x",
            NumberSyntax::Tcl90,
            Some(TclVersion::V9_0)
        ));
        assert!(!is_expr_number(
            "NaN(1)",
            NumberSyntax::Tcl84,
            Some(TclVersion::V8_4)
        ));
        assert!(!is_expr_number(
            "Infinityeq",
            NumberSyntax::Tcl90,
            Some(TclVersion::V9_0)
        ));
    }

    /// The three ways a consumer can stand in relation to the release, and what
    /// each answers for a spelling the releases read differently.
    #[test]
    fn numbers_variants_answer_their_own_use_case() {
        // A named target reads under exactly that release.
        assert_eq!(
            Numbers::Target(NumberSyntax::Tcl85).parse_wide("010"),
            Some(8)
        );
        assert_eq!(
            Numbers::Target(NumberSyntax::Tcl90).parse_wide("010"),
            Some(10)
        );
        // No release in hand: the readings disagree, so no answer.
        assert_eq!(Numbers::Unknown.parse_wide("010"), None);
        // ...but a spelling every release agrees on still resolves.
        assert_eq!(Numbers::Unknown.parse_wide("007"), Some(7));
        assert_eq!(Numbers::Unknown.parse_wide("0x1f"), Some(31));
        // The runtime variant follows whatever was installed.
        let restore = runtime_syntax();
        set_runtime_syntax(NumberSyntax::Tcl85);
        assert_eq!(Numbers::Runtime.parse_wide("010"), Some(8));
        set_runtime_syntax(NumberSyntax::Tcl90);
        assert_eq!(Numbers::Runtime.parse_wide("010"), Some(10));
        set_runtime_syntax(restore);
    }

    /// The two abstention directions are genuinely different, and a consumer
    /// picking the wrong one is the silent failure this API exists to prevent.
    #[test]
    fn unknown_abstains_in_both_directions() {
        // "Is this provably a number?" — `08` is one only from 9.0, so no.
        assert!(!Numbers::Unknown.is_whole_number("08"));
        // "Could this still be a number?" — yes, on 9.0.
        assert!(Numbers::Unknown.is_number_in_any_release("08"));
        // Something no release reads as a number answers false both ways.
        assert!(!Numbers::Unknown.is_whole_number("0o8"));
        assert!(!Numbers::Unknown.is_number_in_any_release("0o8"));
        // And something every release reads answers true both ways.
        assert!(Numbers::Unknown.is_whole_number("42"));
        assert!(Numbers::Unknown.is_number_in_any_release("42"));
    }

    /// `of_profile` / `of_dialect_name` are the only sanctioned ways to get from
    /// a dialect to a grammar, so they must agree with the profile catalogue.
    #[test]
    fn resolvers_agree_with_the_profile_catalogue() {
        assert_eq!(
            NumberSyntax::of_dialect_name(Some("tcl8.6")),
            NumberSyntax::Tcl85
        );
        assert_eq!(
            NumberSyntax::of_dialect_name(Some("tcl9.0")),
            NumberSyntax::Tcl90
        );
        // No dialect named falls to the permissive default, not to 8.x.
        assert_eq!(NumberSyntax::of_dialect_name(None), NumberSyntax::default());
        assert_eq!(NumberSyntax::of_profile(None), NumberSyntax::default());
    }

    /// The ambient's `const` initialiser cannot call `Default`, so it names the
    /// variant directly — two places asserting one fact. This pins them.
    #[test]
    fn numbers_default_matches_the_ambient_initialiser() {
        // A fresh thread has never had `set_runtime_syntax` called on it, so it
        // still holds the initialiser's value.
        let ambient = std::thread::spawn(runtime_syntax).join().unwrap();
        assert_eq!(ambient, NumberSyntax::default());
    }
}
