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

//! `string is`: the class table, the membership tests behind its fold and the type each class proves.

use super::{ArgValue, TclType, TclVersion};
use tcl_syntax::number;
use tcl_syntax::number::{Number, NumberSyntax, Numbers};

/// `string is class ?-strict? ?-failindex var? string`.
/// Constant-folds the **Tcl-faithful** classes
/// the optimiser can decide soundly.
///
/// Deliberately follows Tcl's classification, not a naive
/// letter-case predicate whose semantics diverge: e.g. a check that
/// passes `"abc1"` as "lower" disagrees with `string is lower abc1`,
/// which is `0` (Tcl requires *every* char to be a lowercase letter;
/// a digit fails).  The ASCII character
/// classes here apply the predicate to every char and **bail on
/// non-ASCII input** (Unicode class membership isn't modelled — a
/// missed fold, never a wrong one).  `ascii` is the membership test
/// itself, so it is defined for any input.  `boolean` / `true` /
/// `false` use the exact `Tcl_GetBoolean` keyword + unique-prefix set
/// (so `t` / `ye` / `of` resolve, `o` — ambiguous between on/off —
/// does not).  `list` folds (a backslash-free string's well-formedness is
/// unambiguous).  `integer` and `double` fold over their dialect-invariant
/// subsets (see [`string_is_integer`] / [`string_is_double`]); `entier` /
/// `wideinteger` / `dict` stay deferred — they *raise* in old dialects
/// (8.4 / 8.4 + 8.5 / pre-9.0 respectively), so no dialect-agnostic fold can
/// be sound (it would turn an error into a value).
pub(crate) fn fold_is(args: &[&str], version: Option<TclVersion>) -> Option<String> {
    if args.len() < 2 {
        return None;
    }
    let class = args[0];
    // Tcl grammar: `string is class ?-strict? ?-failindex var? str` — the
    // **last** argument is always the value under test; the args between the
    // class and it are options.  Verified against tclsh 8.4 → 9.0:
    // `string is integer -7` reads `-7` as the *value* (→ 1), and a lone
    // `string is integer -strict` reads `-strict` as the value (→ 0), not an
    // option.  Taking the last arg as the value (rather than treating every
    // `-`-leading arg as an option, as a naive scan would) lets a negative
    // value fold and matches real Tcl.
    let s = *args.last()?;
    let mut strict = false;
    for opt in &args[1..args.len() - 1] {
        match *opt {
            "-strict" => strict = true,
            // `-failindex var` writes a variable (never folded); an unknown
            // option is ambiguous — bail in both cases.
            _ => return None,
        }
    }

    // A class that *raises* in this dialect can't fold to any value — not even
    // the empty-string shortcut below (`string is dict ""` raises before 9.0).
    if !class_available(class, version) {
        return None;
    }

    // The empty string is a member of every available class in non-strict mode
    // and a member of none in strict mode.
    if s.is_empty() {
        return Some(if strict { "0" } else { "1" }.to_owned());
    }

    let member = match class {
        "alpha" => ascii_all(s, |c| c.is_ascii_alphabetic())?,
        "alnum" => ascii_all(s, |c| c.is_ascii_alphanumeric())?,
        "digit" => ascii_all(s, |c| c.is_ascii_digit())?,
        "lower" => ascii_all(s, |c| c.is_ascii_lowercase())?,
        "upper" => ascii_all(s, |c| c.is_ascii_uppercase())?,
        "xdigit" => ascii_all(s, |c| c.is_ascii_hexdigit())?,
        "space" => ascii_all(s, is_tcl_space)?,
        "control" => ascii_all(s, |c| c.is_ascii_control())?,
        "graph" => ascii_all(s, |c| c.is_ascii_graphic())?,
        "print" => ascii_all(s, |c| c.is_ascii_graphic() || c == ' ')?,
        "punct" => ascii_all(s, |c| c.is_ascii_punctuation())?,
        "wordchar" => ascii_all(s, |c| c.is_ascii_alphanumeric() || c == '_')?,
        // `ascii` is the test itself — defined for any input.
        "ascii" => s.is_ascii(),
        "boolean" => tcl_bool(s).is_some(),
        "true" => tcl_bool(s) == Some(true),
        "false" => tcl_bool(s) == Some(false),
        // `list`: well-formed Tcl list.  `split_list` returns `Some`
        // only for a valid (backslash-free) list and `None` for a
        // malformed one — but it conservatively rejects *any* backslash,
        // and a backslash-bearing string can still be a valid list
        // (`a\ b`), so a `None` there is ambiguous.  Sound resolution:
        // a backslash-free string folds (`Some` → 1, `None` → 0); a
        // backslash-bearing one bails.
        "list" => crate::InvocationDialect::for_version(version?)
            .read_list(s)
            .is_ok(),
        // Version-aware number classes (availability gated by
        // `class_available` above); each decides 1/0/bail for `version`.
        "integer" => return is_integer_class(s, version),
        "wideinteger" => return is_wide_class(s, version),
        "entier" => return is_entier_class(s, version),
        "dict" => crate::InvocationDialect::for_version(version?)
            .read_list(s)
            .is_ok_and(|words| words.len().is_multiple_of(2)),
        "double" => return string_is_double(s, version),
        _ => return None, // unknown class
    };
    Some(if member { "1" } else { "0" }.to_owned())
}

/// The type every value a `string is` test accepts holds once it has: what
/// the true edge of a branch on `[string is CLASS ?-strict? $x]` proves of
/// `x` (`docs/design/compiler/value-transfers.md` § *Predicate refinement*),
/// a type and never a value. `args` are the words after `is` — the class, as
/// Tcl reads it with its abbreviations, the options, and the value last, as
/// [`fold_is`] reads them.
///
/// The test converts the value it accepts, so the type is the representation
/// the value then holds (`tcl::unsupported::representation`, tclsh 8.6 to
/// 9.1): an integer class (`integer`, `entier`, `wideinteger`) leaves an
/// integer, `double` an integer or a double, so `Numeric`, each only under
/// `-strict`, since without it the empty string, which none converts, passes
/// every class; `dict` leaves a dictionary, the empty one included. `list`
/// leaves the empty string a pure string, and `boolean`, `true` and `false`
/// leave `0` and `1` integers, so neither proves a type, nor does a character
/// class. `-failindex` writes a variable, and an option spelled any other way
/// than `-strict` is one this reading does not decide, so either proves
/// nothing.
#[must_use]
pub fn string_is_member_type(args: &[&str]) -> Option<TclType> {
    let (class, rest) = args.split_first()?;
    let (_value, options) = rest.split_last()?;
    let mut strict = false;
    for option in options {
        match *option {
            "-strict" => strict = true,
            _ => return None,
        }
    }
    let class =
        tcl_cmd_core::string_is::resolve_class(class, tcl_dialect::TclVersion::V9_1).ok()?;
    match class {
        "integer" | "entier" | "wideinteger" if strict => Some(TclType::Int),
        "double" if strict => Some(TclType::Numeric),
        "dict" => Some(TclType::Dict),
        _ => None,
    }
}

/// Conditional C class type under the independently selected invocation
/// family and release. Lexical numeral grammar supplies no class availability,
/// and this projection supplies no physical cache, frame or store effects.
#[must_use]
pub fn string_is_member_type_for_dialect(
    args: &[&str],
    dialect: crate::InvocationDialect,
) -> Option<TclType> {
    let point = dialect.execution_point()?;
    if point.family() != tcl_dialect::model::Family::Tcl {
        return None;
    }
    let version = point.tcl_version();
    // The class word abbreviates against the release's own class table.
    let class = tcl_cmd_core::string_is::resolve_class(
        args.first().copied()?,
        version.unwrap_or(tcl_dialect::TclVersion::V9_1),
    )
    .ok()?;
    class_available(class, version).then(|| string_is_member_type(args))?
}

/// Whether `string is class` is a *defined* operation in the target version
/// (it doesn't *raise*).  `wideinteger` requires 8.5+, `entier` 8.6+, and
/// `dict` is **9.0-only** (matching the gating below); an unknown version
/// (`None`) treats all three as unavailable, since one of the dialects it
/// could stand for raises.  Every other class exists in every release.
fn class_available(class: &str, version: Option<TclVersion>) -> bool {
    // Single source of truth: the `IS_CLASSES` arg-value table carries each
    // class's introducing release (`min_tcl`, the §6 argument-DSL rung) —
    // the same data the analyser's version diagnostics read.
    match IS_CLASSES.iter().find(|av| av.value == class) {
        Some(av) => match av.min_tcl {
            Some(min) => version.is_some_and(|v| v >= min),
            None => true,
        },
        None => true,
    }
}

/// The magnitude bounds the `string is` integer classes use across releases.
const U32_MAX: u128 = 4_294_967_295;

const I64_MAX: u128 = 9_223_372_036_854_775_807;

const U64_MAX: u128 = 18_446_744_073_709_551_615;

/// Classification of a `string is <intclass>` value as a plain decimal.
enum IntForm {
    /// Definitely not an integer in any form (→ `"0"` for an available class).
    NotInteger,
    /// A leading-zero / `0x` / `0o` / `0b` / non-ASCII form whose validity or
    /// magnitude is version-dependent — bail.
    Ambiguous,
    /// A plain decimal: `mag` is its absolute value, or `None` when it exceeds
    /// `u128` (astronomically large).
    Decimal { neg: bool, mag: Option<u128> },
}

/// Classify `s` as a plain decimal integer.  Restricting to plain decimals
/// (no leading zero, no `0x`/`0o`/`0b`) makes the *form* validity
/// version-independent — only the magnitude cap varies — so the per-class
/// handlers compare `mag` against their version's bound.
fn classify_int_form(s: &str) -> IntForm {
    if !s.is_ascii() {
        return IntForm::Ambiguous;
    }
    let t = s.trim();
    if t.is_empty() {
        return IntForm::NotInteger; // whitespace-only
    }
    let (neg, body) = if let Some(rest) = t.strip_prefix('-') {
        (true, rest)
    } else {
        (false, t.strip_prefix('+').unwrap_or(t))
    };
    let b = body.as_bytes();
    if b.is_empty() {
        return IntForm::NotInteger; // a lone sign
    }
    if b.contains(&b'_') {
        return IntForm::Ambiguous; // Tcl 9 digit separators
    }
    if b[0] == b'0'
        && b.len() >= 2
        && matches!(b[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B' | b'd' | b'D')
    {
        return IntForm::Ambiguous; // Tcl 9 radix prefixes / Tcl 8.x divergence
    }
    if !b.iter().all(u8::is_ascii_digit) {
        return IntForm::NotInteger; // a non-digit with no recognised prefix
    }
    if b.len() > 1 && b[0] == b'0' {
        return IntForm::Ambiguous; // leading zero → octal interpretation in 8.x
    }
    IntForm::Decimal {
        neg,
        mag: body.parse::<u128>().ok(),
    }
}

fn tcl9_integer_number(s: &str) -> Option<Number> {
    if !s.is_ascii() {
        return None;
    }
    Numbers::Target(NumberSyntax::Tcl90).parse_whole(s)
}

/// `string is integer`: known Tcl 9 uses the shared Tcl 9 number parser;
/// older/unknown dialects stay on the conservative plain-decimal subset.
/// A plain decimal of magnitude ≤ `2³²-1` is valid in every release
/// (→ `1`); larger or Tcl-9-only forms bail unless the dialect is known.
fn is_integer_class(s: &str, version: Option<TclVersion>) -> Option<String> {
    if version >= Some(TclVersion::V9_0) {
        return Some(
            if matches!(
                tcl9_integer_number(s),
                Some(Number::Int(_) | Number::Big { .. })
            ) {
                "1"
            } else {
                "0"
            }
            .to_owned(),
        );
    }
    match classify_int_form(s) {
        IntForm::Ambiguous => None,
        IntForm::NotInteger => Some("0".to_owned()),
        IntForm::Decimal { mag, .. } => {
            let within_32 = mag.is_some_and(|m| m <= U32_MAX);
            match version {
                Some(TclVersion::V9_0 | TclVersion::V9_1) => Some("1".to_owned()),
                Some(_) => Some(if within_32 { "1" } else { "0" }.to_owned()),
                None => within_32.then(|| "1".to_owned()),
            }
        }
    }
}

/// `string is wideinteger` (available 8.5+, gated by [`class_available`]):
/// known Tcl 9 uses the shared Tcl 9 number parser and the signed-64-bit
/// bound. 8.5/8.6 accept the unsigned-64-bit range; negative values bail
/// there because that signed/unsigned boundary differs by dialect.
fn is_wide_class(s: &str, version: Option<TclVersion>) -> Option<String> {
    if version >= Some(TclVersion::V9_0) {
        return Some(
            if matches!(tcl9_integer_number(s), Some(Number::Int(_))) {
                "1"
            } else {
                "0"
            }
            .to_owned(),
        );
    }
    match classify_int_form(s) {
        IntForm::Ambiguous => None,
        IntForm::NotInteger => Some("0".to_owned()),
        IntForm::Decimal { neg, mag } => {
            if neg {
                return None;
            }
            let cap = if version >= Some(TclVersion::V9_0) {
                I64_MAX
            } else {
                U64_MAX
            };
            Some(
                if mag.is_some_and(|m| m <= cap) {
                    "1"
                } else {
                    "0"
                }
                .to_owned(),
            )
        }
    }
}

/// `string is entier` (available 8.6+, gated by [`class_available`]):
/// known Tcl 9 uses the shared Tcl 9 number parser. Older known dialects
/// accept the conservative plain-decimal subset at arbitrary precision.
fn is_entier_class(s: &str, version: Option<TclVersion>) -> Option<String> {
    if version >= Some(TclVersion::V9_0) {
        return Some(
            if matches!(
                tcl9_integer_number(s),
                Some(Number::Int(_) | Number::Big { .. })
            ) {
                "1"
            } else {
                "0"
            }
            .to_owned(),
        );
    }
    match classify_int_form(s) {
        IntForm::Ambiguous => None,
        IntForm::NotInteger => Some("0".to_owned()),
        IntForm::Decimal { .. } => Some("1".to_owned()),
    }
}

/// `string is double` over the **dialect-invariant** subset.  `s` is the
/// non-empty value under test (the empty string is handled by [`fold_is`]).
///
/// `string is double` is remarkably consistent across Tcl 8.4 → 9.0, and on
/// the decimal / scientific / `Inf` / `NaN` forms Rust's `f64::from_str`
/// (after trimming) agrees with it **exactly** — verified over a broad matrix
/// against `tclsh8.4`/`8.5`/`8.6`/`9.0` during development, e.g. `5.e3`, `.5`,
/// `1.`, `+.5`, `1E-10`, `inf`, `nan` → `1`; `1e`, `e5`, `.`, `1.2.3` → `0`.
/// The few forms where they part ways are version-divergent anyway, so they
/// **bail**:
///
/// * a `_` digit separator — accepted only by Tcl 9 (`1_000` → `0` on 8.x,
///   `1` on 9.0), rejected by Rust;
/// * a `0x` / `0o` / `0b` prefix — `0x…` is a valid double in every version
///   but Rust's parser rejects it, `0o…` / `0b…` raise in 8.4, and a
///   hex-float (`0x1.8p3`) flips 8.4 (`1`) ↔ 8.5+ (`0`);
/// * a `(` — the `nan(payload)` form, which Tcl accepts but Rust rejects;
/// * a non-ASCII value (so the `.trim()` whitespace set matches Tcl's).
///
/// A decimal spelling outside the double range (`1e400`, `1e-400`, a
/// subnormal) is a release question the number owner answers
/// ([`NumberSyntax::string_is_double_rejects_range_error`]): `0` on 8.4, `1`
/// from 8.5, and a bail when `version` names no core release (Jim refuses it
/// too, but has no `TclVersion`).
fn string_is_double(s: &str, version: Option<TclVersion>) -> Option<String> {
    if !s.is_ascii() {
        return None;
    }
    let t = s.trim();
    if t.contains('_') || t.contains('(') {
        return None;
    }
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let b = body.as_bytes();
    if b.len() >= 2 && b[0] == b'0' && matches!(b[1], b'x' | b'X' | b'o' | b'O' | b'b' | b'B') {
        return None;
    }
    // For every remaining ASCII form, Rust's `f64` parser and Tcl's
    // `string is double` agree across 8.4 → 9.0 — except a spelling outside
    // the double range, which 8.4 and Jim refuse and 8.5 onward accept.
    if t.parse::<f64>().is_err() {
        return Some("0".to_owned());
    }
    if !number::is_out_of_range_double(t, NumberSyntax::Tcl90) {
        return Some("1".to_owned());
    }
    let numbers = version.map_or(Numbers::Unknown, |v| Numbers::Target(v.number_syntax()));
    let rejected = numbers.resolve(NumberSyntax::string_is_double_rejects_range_error)?;
    Some(if rejected { "0" } else { "1" }.to_owned())
}

/// Apply an ASCII char predicate to every char of `s`, bailing (`None`)
/// on any non-ASCII char — Unicode class membership isn't modelled here.
fn ascii_all(s: &str, pred: impl Fn(char) -> bool) -> Option<bool> {
    if !s.is_ascii() {
        return None;
    }
    Some(s.chars().all(pred))
}

/// Tcl's whitespace set for `string is space` (`Tcl_UniCharIsSpace` on
/// ASCII): space, tab, newline, CR, vertical tab, form feed.  (Rust's
/// `char::is_ascii_whitespace` omits the vertical tab.)
fn is_tcl_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{0b}' | '\u{0c}')
}

/// `Tcl_GetBoolean`: the canonical strict acceptor — `0` / `1` plus the
/// case-insensitive *unique* prefixes of the six boolean words. One home:
/// [`tcl_syntax::boolean::parse_boolean_strict`] (oracle-table-pinned).
fn tcl_bool(s: &str) -> Option<bool> {
    tcl_syntax::boolean::parse_boolean_strict(s)
}

/// Character classes accepted by `string is <class>`.
pub(super) static IS_CLASSES: &[ArgValue] = &[
    ArgValue {
        value: "alnum",
        detail: "Any Unicode alphabet or digit character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "alpha",
        detail: "Any Unicode alphabet character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "ascii",
        detail: "Any character with a value less than U+0080 (7-bit ASCII).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "boolean",
        detail: "Any valid boolean value (true/false/yes/no/on/off/0/1).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "control",
        detail: "Any Unicode control character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "dict",
        detail: "Any proper dict structure, with optional surrounding whitespace.",
        min_tcl: Some(TclVersion::V9_0),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "digit",
        detail: "Any Unicode digit character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "double",
        detail: "Any valid floating-point number.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "entier",
        detail: "An arbitrary-size integer. On Tcl 8.6, where integer itself was still 32-bit-bounded, entier was the only arbitrary-size class; from Tcl 9.0, where integer itself became arbitrary-size, entier is a plain synonym for it.",
        min_tcl: Some(TclVersion::V8_6),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "false",
        detail: "Any valid boolean false value.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "graph",
        detail: "Any Unicode printing character, except space.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "integer",
        detail: "Any valid integer, with optional surrounding whitespace. Bounded to 32 bits through Tcl 8.6; arbitrary size from Tcl 9.0 onward.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "list",
        detail: "Any proper list structure, with optional surrounding whitespace.",
        // Absent from the Tcl 8.4 manpage's class table (18 classes, no
        // `list`); present from 8.5 (19 classes, `list` added). `string is
        // list ...` raises "bad class" under an 8.4 dialect rather than
        // returning 0/1.
        min_tcl: Some(TclVersion::V8_5),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "lower",
        detail: "Any Unicode lower case alphabet character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "print",
        detail: "Any Unicode printing character, including space.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "punct",
        detail: "Any Unicode punctuation character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "space",
        detail: "Any Unicode whitespace character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "true",
        detail: "Any valid boolean true value.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "upper",
        detail: "Any upper case alphabet character.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "wideinteger",
        detail: "Any valid wide integer, with optional surrounding whitespace. From Tcl 9.0 this is bounded to the signed 64-bit range; Tcl 8.5/8.6 accept a wider positive magnitude (up to 2^64-1).",
        min_tcl: Some(TclVersion::V8_5),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "wordchar",
        detail: "Any Unicode word character (alphanumeric + connector punctuation).",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "xdigit",
        detail: "Any hexadecimal digit character (0-9, A-F, a-f).",
        ..ArgValue::DEFAULT
    },
];
