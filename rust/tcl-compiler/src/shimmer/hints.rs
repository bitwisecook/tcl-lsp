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

//! Registry look-up helpers for shimmer analysis.
//!
//! - [`arg_shimmer_type`] — expected `TclType` for an argument position
//!   when that position has `shimmers = true` in the registry.
//! - [`is_numeric_compatible`] — true when two types are interchangeable
//!   in Tcl's arithmetic/boolean contexts.
//! - [`is_uncommitted_first_conversion`] — true when reading a value as a
//!   different type is the *first* conversion of an uncommitted (pure) value,
//!   which Tcl performs for free rather than a genuine shimmer.

use std::borrow::Cow;

use tcl_registry::{CommandRegistry, TclType};
use tcl_syntax::boolean::parse_boolean_word;
use tcl_syntax::list::split_list;
use tcl_syntax::number::{self, NumberSyntax, ParseFlags};

use crate::analyses::{ConstValue, LatticeValue};

/// Return the expected `TclType` for argument `arg_index` of `command`
/// when that argument position is tagged `shimmers = true` in the registry.
///
/// `arg_index` is 0-based from the first argument after the command name,
/// matching `Statement::Call { args, .. }`.  For subcommand-dispatching
/// commands (e.g. `string`, `dict`), `args[0]` names the subcommand and
/// the remaining args are offset by one; the subcommand's own `arg_types`
/// uses 0-based indexing from after the subcommand word, so the conversion
/// is `sub_index = arg_index - 1`.
///
/// Returns `None` when:
/// - The command is absent from the registry.
/// - The argument position has no type hint.
/// - The type hint has `shimmers = false`.
#[must_use]
pub fn arg_shimmer_type(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    arg_index: usize,
) -> Option<TclType> {
    arg_shimmer_expectation(registry, command, args, arg_index).map(|e| e.expected)
}

/// A shimmering argument position's expectation: the intrep the operation
/// installs, plus the current intreps it reads directly *without* converting
/// (the registry's `ArgTypeHint::transparent_from` — e.g. `string length`'s
/// pure-byte-array fast path).
#[derive(Debug, Clone, Copy)]
pub struct ShimmerExpectation {
    /// The intrep the operation installs on the operand.
    pub expected: TclType,
    /// Current intreps that pass through unconverted despite differing from
    /// [`Self::expected`] — no shimmer for an operand already in one of these.
    pub transparent_from: &'static [TclType],
}

impl ShimmerExpectation {
    /// Whether an operand currently holding `current` is left untouched.
    #[must_use]
    pub fn is_transparent_from(&self, current: TclType) -> bool {
        self.transparent_from.contains(&current)
    }
}

/// Like [`arg_shimmer_type`] but returning the full [`ShimmerExpectation`]
/// (expected type + transparency list) for consumers that must suppress the
/// warning on transparent current intreps.
#[must_use]
pub fn arg_shimmer_expectation(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    arg_index: usize,
) -> Option<ShimmerExpectation> {
    let spec = registry.get(command)?;
    let expectation = |h: &tcl_registry::hooks::ArgTypeHint| {
        if h.shimmers {
            h.expected.map(|expected| ShimmerExpectation {
                expected,
                transparent_from: h.transparent_from,
            })
        } else {
            None
        }
    };

    // Subcommand dispatch: spec has subcommands and args[0] names one.
    if !spec.subcommands.is_empty() {
        let sub_name = args.first().copied()?;
        let sub = spec.resolve_subcommand(sub_name)?;
        // arg_index 0 = subcommand word; subtract 1 for the sub-relative
        // index, then subtract the leading declared option words so the
        // static positional hints stay aligned under a `?-nocase?`-style
        // prefix (`string map -nocase $mapping $subject` must resolve
        // `$mapping` to positional 0, not 1). A query landing *inside* the
        // option prefix (`rel < skip`) is an option word — never a hint.
        let rel = arg_index.checked_sub(1)?;
        let skip = sub.leading_option_word_count(args.get(1..).unwrap_or(&[]));
        let sub_idx = u8::try_from(rel.checked_sub(skip)?).ok()?;
        return sub
            .arg_types
            .iter()
            .find(|(i, _)| *i == sub_idx)
            .and_then(|(_, h)| expectation(h));
    }

    let needle = u8::try_from(arg_index).ok()?;
    spec.arg_types
        .iter()
        .find(|(i, _)| *i == needle)
        .and_then(|(_, h)| expectation(h))
}

/// Return `true` when the two types are numerically compatible — i.e. a
/// value of type `current` used in a context expecting `expected` would
/// not cause a shimmer.
///
/// Tcl's runtime treats `Int`, `Boolean`, and `Numeric` as interchangeable
/// in arithmetic and boolean contexts; no intrep conversion is needed.
#[must_use]
pub fn is_numeric_compatible(current: TclType, expected: TclType) -> bool {
    use TclType::{Boolean, Int, Numeric};
    if current == expected {
        return true;
    }
    matches!(
        (current, expected),
        (Int | Numeric, Boolean) | (Boolean | Numeric, Int) | (Int | Boolean, Numeric)
    )
}

/// Whether reading a value of committed intrep `current` at a position that
/// expects `expected` is a **free first conversion of an uncommitted value**,
/// not a genuine shimmer.
///
/// The Tcl object model — oracle-verified and set out in
/// [`docs/design/contracts/shimmer-reference-behaviour.md`](../../../../docs/design/contracts/shimmer-reference-behaviour.md)
/// ("When shimmering does NOT occur → Pure string objects (`typePtr == NULL`) —
/// first type assignment is not a shimmer") — is that a value whose intrep is
/// not yet committed pays nothing the *first* time it is read as another type it
/// is a valid instance of. A bare literal (`set l {a b c}`), an interpolated
/// string, and any string-producing command (`string trim`, `format`, `join`)
/// all yield such a pure value: `foreach $l` / `lindex $l 0` merely parses it
/// into a list intrep once, losslessly. Only a **committed** container / object
/// / byte-array intrep — produced by `[list]`, `[dict create]`, an object
/// factory, or `binary format` — genuinely re-represents when read as another
/// type, which is the real shimmer this analysis exists to flag.
///
/// `current` classifies committed-ness generically, never by command name:
/// - [`TclType::String`] is documented "pure string, no cached intrep" — always
///   uncommitted.
/// - A numeric `current` ([`TclType::Int`] / [`TclType::Double`] /
///   [`TclType::Boolean`] / [`TclType::Numeric`]) is uncommitted only when the
///   value is a compile-time constant — a constant-folded literal push, still
///   `typePtr == NULL`. A *runtime* `expr` / `incr` result is a committed
///   numeric intrep whose reconversion is a genuine shimmer.
/// - [`TclType::List`] / [`TclType::Dict`] / [`TclType::ByteArray`] /
///   [`TclType::Object`] / [`TclType::Channel`] are always committed.
///
/// `const_value` is the SCCP constant for the value at this use, when known; its
/// string form(s) drive the "valid instance of `expected`" test. A [`ConstSet`]
/// (a merge of several literals, e.g. a phi of two `set` arms) is still a set of
/// pure literals, so it is uncommitted and free precisely when **every** member
/// is a valid instance. A pure value that is *not* a valid instance (e.g. `set s
/// hello; expr {$s + 1}`, or a bad-syntax list) is **not** suppressed — its
/// conversion would fail at runtime, which the mismatch warning is right to
/// surface.
///
/// [`ConstSet`]: LatticeValue::ConstSet
#[must_use]
pub fn is_uncommitted_first_conversion(
    current: TclType,
    expected: TclType,
    const_value: Option<&LatticeValue>,
    numbers: NumberSyntax,
) -> bool {
    is_pure_intrep(current, const_value) && is_valid_instance_of(expected, const_value, numbers)
}

/// Whether a value of lattice type `current` (with SCCP constant `const_value`,
/// if any) has an **uncommitted** intrep — a pure string with no cached typed
/// representation (`typePtr == NULL`).
///
/// `String` is always uncommitted (documented "pure string, no cached intrep").
/// A numeric type is uncommitted only when the value is a compile-time constant
/// (a constant-folded literal push); a runtime `expr` / `incr` result is a
/// committed numeric intrep. `List` / `Dict` / `Object` / `ByteArray` /
/// `Channel` are always committed. See [`is_uncommitted_first_conversion`] for
/// the full contract.
#[must_use]
pub fn is_pure_intrep(current: TclType, const_value: Option<&LatticeValue>) -> bool {
    match current {
        TclType::String => true,
        TclType::Int | TclType::Double | TclType::Boolean | TclType::Numeric => {
            const_value.and_then(const_string_forms).is_some()
        }
        TclType::List | TclType::Dict | TclType::ByteArray | TclType::Object | TclType::Channel => {
            false
        }
    }
}

/// Whether a pure value with SCCP constant `const_value` is a **valid instance**
/// of `expected` — its first conversion would succeed and cost nothing. A
/// constant (or constant set) must have *every* member be a valid instance; a
/// non-constant value is presumed valid only for the permissive [`TclType::List`]
/// (any string is a list) and keeps its warning for the stricter targets.
#[must_use]
pub fn is_valid_instance_of(
    expected: TclType,
    const_value: Option<&LatticeValue>,
    numbers: NumberSyntax,
) -> bool {
    match const_value.and_then(const_string_forms) {
        Some(forms) => forms
            .iter()
            .all(|s| string_is_valid_instance(Some(s), expected, numbers)),
        None => string_is_valid_instance(None, expected, numbers),
    }
}

/// The canonical Tcl string form(s) of an SCCP constant value, or `None` when it
/// is not known at compile time (unknown / top). A single [`Const`] yields one
/// form; a [`ConstSet`] — a small set of possible literals from a merge — yields
/// each, so a caller can require *all* of them to satisfy a predicate.
///
/// [`Const`]: LatticeValue::Const
/// [`ConstSet`]: LatticeValue::ConstSet
fn const_string_forms(value: &LatticeValue) -> Option<Vec<Cow<'_, str>>> {
    match value {
        LatticeValue::Const(c) => Some(vec![const_value_string(c)]),
        LatticeValue::ConstSet(cs) => Some(cs.iter().map(const_value_string).collect()),
        LatticeValue::Unknown | LatticeValue::Overdefined => None,
    }
}

/// The canonical Tcl string form of one SCCP constant.
fn const_value_string(value: &ConstValue) -> Cow<'_, str> {
    match value {
        ConstValue::String(s) => Cow::Borrowed(s),
        ConstValue::Int(i) => Cow::Owned(i.to_string()),
        ConstValue::Bool(b) => Cow::Borrowed(if *b { "1" } else { "0" }),
        ConstValue::Float(f) => Cow::Owned(number::format_double(*f)),
    }
}

/// Whether the pure-string form `s` (`None` ⇒ the value is not a compile-time
/// constant) is a valid instance of `expected`, so its first conversion would
/// succeed and cost nothing.
///
/// A [`TclType::List`] accepts virtually any string, so a non-constant pure
/// value is presumed a valid (free) list promotion; the stricter targets — a
/// [`TclType::Dict`]'s even length and the numeric tower — need the constant to
/// prove validity, and a value that cannot be proven valid keeps its warning.
fn string_is_valid_instance(s: Option<&str>, expected: TclType, numbers: NumberSyntax) -> bool {
    let is_number = |s: &str| {
        number::parse_whole_with(s, number::ParseFlags::for_syntax(numbers)).is_some()
    };
    match expected {
        TclType::List => s.is_none_or(|s| split_list(s).is_ok()),
        TclType::Dict => s.is_some_and(|s| split_list(s).is_ok_and(|els| els.len() % 2 == 0)),
        TclType::Int => s.is_some_and(|s| is_valid_integer(s, numbers)),
        TclType::Double | TclType::Numeric => s.is_some_and(is_number),
        TclType::Boolean => {
            s.is_some_and(|s| parse_boolean_word(s).is_some() || is_number(s))
        }
        TclType::String | TclType::ByteArray | TclType::Object | TclType::Channel => false,
    }
}

/// Whether `s` is a whole Tcl integer literal (`Tcl_GetWideIntFromObj` /
/// `TCL_PARSE_INTEGER_ONLY`): decimal, `0x`/`0o`/`0b`/`0d` radix, or a bignum —
/// but not a fractional/exponent form, which `incr`/index positions reject.
fn is_valid_integer(s: &str, numbers: NumberSyntax) -> bool {
    matches!(
        number::parse_whole_with(
            s,
            ParseFlags {
                integer_only: true,
                ..ParseFlags::for_syntax(numbers)
            },
        ),
        Some(number::Number::Int(_) | number::Number::Big { .. })
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::CommandRegistry;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn arg_shimmer_type_incr_varname() {
        // `incr varName` — arg 0 expects Int with shimmers=true.
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "incr", &["myvar"], 0),
            Some(TclType::Int)
        );
    }

    #[test]
    fn arg_shimmer_type_lindex_list() {
        // `lindex list ?index...?` — arg 0 expects List with shimmers=true.
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "lindex", &["$mylist", "0"], 0),
            Some(TclType::List)
        );
    }

    #[test]
    fn arg_shimmer_type_subcommand_string_index_charindex() {
        // `string index string charIndex` — sub arg 1 (overall arg 2) expects Int.
        // args: ["index", "$s", "3"] → arg_index=2 → sub_idx=1 → Int
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "string", &["index", "$s", "3"], 2),
            Some(TclType::Int)
        );
    }

    #[test]
    fn arg_shimmer_type_subcommand_arg0_is_subcommand_word() {
        // arg_index=0 is the subcommand word itself — should return None (sub_idx underflow).
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "string", &["index", "$s", "3"], 0),
            None
        );
    }

    #[test]
    fn arg_shimmer_type_unknown_command_is_none() {
        let r = registry();
        assert_eq!(arg_shimmer_type(&r, "_nonexistent_", &[], 0), None);
    }

    #[test]
    fn arg_shimmer_type_binary_decode_data_is_arg1_not_format_keyword() {
        // `binary decode format data` — sub arg 1 (overall arg 2) is `data`,
        // read via its string rep only: dual-ported, the intrep is KEPT
        // (tclsh-verified: `set d 4142; binary decode hex $d` leaves `d` an
        // int), so it is no longer a shimmer position. Sub arg 0 (the
        // `format` keyword, e.g. "hex") never carried the hint.
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "binary", &["decode", "hex", "$data"], 2),
            None,
            "reading the string rep is not a shimmer (dual-porting)"
        );
        assert_eq!(
            arg_shimmer_type(&r, "binary", &["decode", "hex", "$data"], 1),
            None,
            "the 'format' keyword slot must not carry the shimmer hint"
        );
        // `binary encode` genuinely installs the byte-array intrep on `data`.
        assert_eq!(
            arg_shimmer_type(&r, "binary", &["encode", "hex", "$data"], 2),
            Some(TclType::ByteArray)
        );
    }

    #[test]
    fn arg_shimmer_expectation_string_length_transparent_bytearray() {
        // `string length` installs the string intrep on its subject
        // (tclsh-verified: an int becomes `string`) — but a pure byte array
        // short-circuits to its byte count and keeps its rep, which the
        // registry expresses via `transparent_from`.
        let r = registry();
        let exp = arg_shimmer_expectation(&r, "string", &["length", "$s"], 1)
            .expect("string length subject must be a shimmer position");
        assert_eq!(exp.expected, TclType::String);
        assert!(exp.is_transparent_from(TclType::ByteArray));
        assert!(!exp.is_transparent_from(TclType::List));
    }

    #[test]
    fn arg_shimmer_type_binary_encode_data_is_arg1_expects_bytearray() {
        // `binary encode format data` — `data` is the raw bytes being
        // encoded, so it expects ByteArray (the reverse of `decode`).
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "binary", &["encode", "hex", "$data"], 2),
            Some(TclType::ByteArray)
        );
    }

    #[test]
    fn arg_shimmer_type_dict_getd_family_matches_dict_get() {
        // `dict getd`/`getdef`/`getwithdefault` (Tcl 9.0 TIP 342 synonyms of
        // `dict get` with a default) must carry the same Dict shimmer hint
        // as plain `dict get` — they were previously inconsistently
        // `shimmers: false`.
        let r = registry();
        for sub in ["getd", "getdef", "getwithdefault", "get"] {
            assert_eq!(
                arg_shimmer_type(&r, "dict", &[sub, "$d", "k", "default"], 1),
                Some(TclType::Dict),
                "dict {sub} should shimmer its dict argument like dict get",
            );
        }
    }

    /// `string first`/`last` install the string intrep on BOTH the needle
    /// and the haystack (tclsh8.6-verified: `set l [list 1 2 3]; string
    /// first x $l` flips `l`'s representation list→string, and so does the
    /// needle position). The startIndex position keeps its Int hint.
    #[test]
    fn arg_shimmer_expectation_string_first_last_needle_and_haystack() {
        let r = registry();
        for sub in ["first", "last"] {
            for (arg_index, label) in [(1usize, "needle"), (2usize, "haystack")] {
                let exp = arg_shimmer_expectation(&r, "string", &[sub, "$n", "$h", "1"], arg_index)
                    .unwrap_or_else(|| panic!("string {sub} {label} must be a shimmer position"));
                assert_eq!(exp.expected, TclType::String, "string {sub} {label}");
                // 9.0's TclStringFirst/Last keep the rep only for the
                // both-pure-byte-array pair; the positional hint cannot see
                // the sibling operand, so ByteArray is transparent (the
                // 9.0-safe under-approximation — 8.6 actually converts).
                assert!(exp.is_transparent_from(TclType::ByteArray));
                assert!(!exp.is_transparent_from(TclType::List));
            }
            assert_eq!(
                arg_shimmer_type(&r, "string", &[sub, "$n", "$h", "$i"], 3),
                Some(TclType::Int),
                "string {sub} startIndex keeps its Int hint"
            );
        }
    }

    /// `string map`'s mapping takes the dict path only for a pure dict and
    /// otherwise goes through `TclListObjGetElements` — tclsh8.6-verified:
    /// a plain-string mapping flips to `list`, a list mapping stays `list`,
    /// a `dict create` mapping stays `dict`. The former `expected: Dict`
    /// hint was refuted by that probe.
    #[test]
    fn arg_shimmer_expectation_string_map_mapping_is_list_dict_transparent() {
        let r = registry();
        let exp = arg_shimmer_expectation(&r, "string", &["map", "$m", "$s"], 1)
            .expect("string map mapping must be a shimmer position");
        assert_eq!(exp.expected, TclType::List);
        assert!(exp.is_transparent_from(TclType::Dict));
        assert!(!exp.is_transparent_from(TclType::ByteArray));
    }

    /// `string map`'s subject is read via `Tcl_GetUnicodeFromObj` in both
    /// 8.6 and 9.0 — tclsh8.6-verified: a list subject flips list→string
    /// AND a pure byte-array subject flips bytearray→string, so nothing is
    /// transparent.
    #[test]
    fn arg_shimmer_expectation_string_map_subject_converts_bytearray_too() {
        let r = registry();
        let exp = arg_shimmer_expectation(&r, "string", &["map", "$m", "$s"], 2)
            .expect("string map subject must be a shimmer position");
        assert_eq!(exp.expected, TclType::String);
        assert!(!exp.is_transparent_from(TclType::ByteArray));
    }

    /// The `?-nocase?` option shifts `string map`'s positional arguments by
    /// one; the leading-option skip keeps the hints aligned: under
    /// `-nocase` (or C Tcl's accepted unique prefix `-noc`) index 2 is the
    /// mapping and index 3 the subject, while index 1 — the option word
    /// itself — carries no hint.
    #[test]
    fn arg_shimmer_expectation_string_map_nocase_shifts_positions() {
        let r = registry();
        for opt in ["-nocase", "-noc"] {
            let args = ["map", opt, "$m", "$s"];
            assert_eq!(
                arg_shimmer_expectation(&r, "string", &args, 1).map(|e| e.expected),
                None,
                "the {opt} option word must carry no hint"
            );
            assert_eq!(
                arg_shimmer_expectation(&r, "string", &args, 2).map(|e| e.expected),
                Some(TclType::List),
                "mapping under {opt}"
            );
            assert_eq!(
                arg_shimmer_expectation(&r, "string", &args, 3).map(|e| e.expected),
                Some(TclType::String),
                "subject under {opt}"
            );
        }
        // A value-taking option consumes its value word too: `string equal
        // -length $n $a $b` must not resolve $n (index 2) to any positional
        // hint (it is the option's value, not a positional argument).
        assert_eq!(
            arg_shimmer_expectation(&r, "string", &["equal", "-length", "$n", "$a", "$b"], 2)
                .map(|e| e.expected),
            None,
        );
    }

    /// Oracle-verified negatives: subjects read via their string rep only
    /// (dual-ported — the intrep survives) carry NO shimmer hint. tclsh8.6:
    /// a list subject stays `list` through every one of these.
    #[test]
    fn arg_shimmer_expectation_dual_ported_string_subjects_have_no_hint() {
        let r = registry();
        for (sub, subject_index) in [
            ("compare", 1usize),
            ("compare", 2),
            ("equal", 1),
            ("equal", 2),
            ("match", 1),
            ("match", 2),
            ("repeat", 1),
            ("tolower", 1),
            ("toupper", 1),
            ("totitle", 1),
            ("trim", 1),
            ("trimleft", 1),
            ("trimright", 1),
            ("cat", 1),
            ("wordend", 1),
            ("wordstart", 1),
        ] {
            let got =
                arg_shimmer_expectation(&r, "string", &[sub, "$a", "$b", "$c"], subject_index)
                    .map(|e| e.expected);
            assert_eq!(got, None, "string {sub} arg {subject_index}");
        }
    }

    #[test]
    fn is_numeric_compatible_same_type() {
        assert!(is_numeric_compatible(TclType::Int, TclType::Int));
        assert!(is_numeric_compatible(TclType::String, TclType::String));
    }

    #[test]
    fn is_numeric_compatible_int_bool() {
        assert!(is_numeric_compatible(TclType::Int, TclType::Boolean));
        assert!(is_numeric_compatible(TclType::Boolean, TclType::Int));
    }

    #[test]
    fn is_numeric_compatible_int_numeric() {
        assert!(is_numeric_compatible(TclType::Int, TclType::Numeric));
        assert!(is_numeric_compatible(TclType::Numeric, TclType::Int));
    }

    #[test]
    fn is_numeric_compatible_string_int_is_false() {
        assert!(!is_numeric_compatible(TclType::String, TclType::Int));
        assert!(!is_numeric_compatible(TclType::List, TclType::Int));
        assert!(!is_numeric_compatible(TclType::String, TclType::List));
    }

    fn s(text: &str) -> LatticeValue {
        LatticeValue::Const(ConstValue::String(text.to_owned()))
    }

    // --- is_uncommitted_first_conversion: TN (suppress — free first conversion)

    /// TN (issue #940): a pure string literal that is a well-formed list is a
    /// free first conversion — every list-shaped constant is suppressed.
    #[test]
    fn uncommitted_pure_string_valid_list_is_free() {
        for lit in ["10.0 12.0 16.0 24.0", "hello", "", "a b c", "1"] {
            assert!(
                is_uncommitted_first_conversion(TclType::String, TclType::List, Some(&s(lit)), NumberSyntax::Tcl90),
                "pure string {lit:?} used as a list must be a free conversion"
            );
        }
    }

    /// TN: an even-length list constant is a valid dict — free.
    #[test]
    fn uncommitted_pure_string_valid_dict_is_free() {
        assert!(is_uncommitted_first_conversion(TclType::String,
            TclType::Dict,
            Some(&s("a 1 b 2"))
        , NumberSyntax::Tcl90));
    }

    /// TN: a constant-folded numeric literal (`set x 5`) is a pure string, so
    /// using it as a list is free — the define-time numeric shape is irrelevant.
    #[test]
    fn uncommitted_const_int_used_as_list_is_free() {
        let v = LatticeValue::Const(ConstValue::Int(5));
        assert!(is_uncommitted_first_conversion(TclType::Int,
            TclType::List,
            Some(&v)
        , NumberSyntax::Tcl90));
    }

    /// TN: a non-constant pure string (interpolation / string command) is
    /// presumed a valid list — lists accept virtually any string.
    #[test]
    fn uncommitted_nonconst_string_as_list_is_free() {
        assert!(is_uncommitted_first_conversion(TclType::String,
            TclType::List,
            None, NumberSyntax::Tcl90));
    }

    /// TN: every member of a `ConstSet` (a phi of literals) is a valid list, so
    /// the merge is still a free promotion.
    #[test]
    fn uncommitted_constset_all_valid_lists_is_free() {
        let v = LatticeValue::ConstSet(vec![ConstValue::Int(1), ConstValue::String("a b".into())]);
        assert!(is_uncommitted_first_conversion(TclType::Int,
            TclType::List,
            Some(&v)
        , NumberSyntax::Tcl90));
    }

    // --- is_uncommitted_first_conversion: TP (fire — genuine shimmer / error)

    /// TP: a committed container intrep (`current == List`/`Dict`/`ByteArray`)
    /// genuinely re-represents — never a free conversion.
    #[test]
    fn committed_container_is_not_free() {
        for current in [TclType::List, TclType::Dict, TclType::ByteArray] {
            assert!(
                !is_uncommitted_first_conversion(current, TclType::String, Some(&s("1 2 3")), NumberSyntax::Tcl90),
                "committed {current:?} must not be treated as a free conversion"
            );
        }
    }

    /// TP: a pure string that is NOT a valid instance keeps its warning — `set s
    /// hello; expr {$s + 1}` and `incr` on a non-integer both fail at runtime.
    #[test]
    fn pure_string_invalid_instance_is_not_free() {
        assert!(!is_uncommitted_first_conversion(TclType::String,
            TclType::Int,
            Some(&s("hello"))
        , NumberSyntax::Tcl90));
        assert!(!is_uncommitted_first_conversion(TclType::String,
            TclType::Numeric,
            Some(&s("hello"))
        , NumberSyntax::Tcl90));
        // An odd-length list is not a valid dict.
        assert!(!is_uncommitted_first_conversion(TclType::String,
            TclType::Dict,
            Some(&s("a 1 b"))
        , NumberSyntax::Tcl90));
    }

    /// TP: a *runtime* numeric intrep (non-constant `expr`/`incr` result) is
    /// committed — its list conversion is a genuine shimmer.
    #[test]
    fn runtime_numeric_used_as_list_is_not_free() {
        assert!(!is_uncommitted_first_conversion(TclType::Int,
            TclType::List,
            None, NumberSyntax::Tcl90));
        assert!(!is_uncommitted_first_conversion(TclType::Numeric,
            TclType::List,
            Some(&LatticeValue::Overdefined)
        , NumberSyntax::Tcl90));
    }

    /// TP: a malformed-list constant (unmatched brace) is not a valid list.
    #[test]
    fn malformed_list_constant_is_not_free() {
        assert!(!is_uncommitted_first_conversion(TclType::String,
            TclType::List,
            Some(&s("oops {unbalanced"))
        , NumberSyntax::Tcl90));
    }

    /// FN guard: a `ConstSet` where *one* member is not a valid instance must
    /// fire — the merge is only free when every path is valid.
    #[test]
    fn constset_one_invalid_member_is_not_free() {
        let v = LatticeValue::ConstSet(vec![
            ConstValue::String("a 1 b 2".into()),
            ConstValue::String("bad {".into()),
        ]);
        assert!(!is_uncommitted_first_conversion(TclType::String,
            TclType::List,
            Some(&v)
        , NumberSyntax::Tcl90));
    }

    #[test]
    fn is_valid_integer_matches_tcl_forms() {
        for ok in [
            "5",
            "-5",
            "0x1f",
            "0o17",
            "0b101",
            "1_000",
            "999999999999999999999",
        ] {
            assert!(is_valid_integer(ok, NumberSyntax::Tcl90), "{ok:?} should be a valid integer");
        }
        for bad in ["5.0", "hello", "", "1 2", "0x"] {
            assert!(
                !is_valid_integer(bad, NumberSyntax::Tcl90),
                "{bad:?} should not be a valid integer"
            );
        }
    }
}
