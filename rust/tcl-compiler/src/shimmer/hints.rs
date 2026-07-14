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

use tcl_registry::{CommandRegistry, TclType};

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
        // arg_index 0 = subcommand word; subtract 1 for sub-relative index.
        let sub_idx = u8::try_from(arg_index.checked_sub(1)?).ok()?;
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
}
