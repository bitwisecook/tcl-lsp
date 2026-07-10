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

use tcl_registry::{CommandRegistry, TclType, Traits};

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
    let spec = registry.get(command)?;

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
            .and_then(|(_, h)| if h.shimmers { h.expected } else { None });
    }

    let fixed = u8::try_from(arg_index).ok().and_then(|needle| {
        spec.arg_types
            .iter()
            .find(|(i, _)| *i == needle)
            .and_then(|(_, h)| if h.shimmers { h.expected } else { None })
    });
    fixed.or_else(|| loop_list_header_shimmer_type(spec, args, arg_index))
}

/// Return `Some(TclType::List)` for any in-range `arg_index` of a
/// `Traits::LOOP_LIST_HEADER` command (`foreach`, `lmap`).
///
/// Registry-driven and command-name-agnostic — keyed purely on the trait
/// bit, so any future command declaring `LOOP_LIST_HEADER` picks this up for
/// free rather than needing its own hardcoded arm here. The list arguments
/// can't be expressed as fixed-position `arg_types` entries because their
/// count varies with the number of `varList`/`list` pairs; this structural
/// rule is the general form the fixed-index table can't capture.
///
/// This command shape reaches the shimmer pass in two different `args`
/// encodings, both of which this rule must cover:
///
/// - **Opaque / non-inlined calls** (`cfg_builder::lower_foreach_dispatch`'s
///   non-inlined arm, taken for namespace-qualified loop vars or when body
///   inlining is off): `args` is the literal source shape, `varList list
///   ?varList list ...? body` — *every* position through the last `list` is
///   genuinely List-shimmering, not just the `list` slots: `Tcl_ForeachObjCmd`
///   calls `Tcl_ListObjGetElements` on **both** the `varList` grouping words
///   and the data `list` words (splitting `varList` into its member variable
///   names requires exactly the same list-object conversion). Only the final
///   `body` word is not — but a `$var` used as a foreach body, while
///   syntactically legal, is not a pattern real Tcl code uses, so the
///   (already narrow, since `check_invocation`/`record_use_targets` only
///   ever act on a `$`-prefixed pure-var-ref word to begin with) residual
///   risk of over-claiming that one slot is negligible next to the false
///   negatives from excluding the `varList` slots would otherwise cause.
/// - **Inlined calls** (the default; `cfg_builder::cfg_lower::lower_foreach`'s
///   synthetic `var_def` `Statement::Call`): `args` is a *different*,
///   list-only encoding — one entry per iterator's data list, no `varList`
///   words and no `body` at all (`iterators.iter().map(|it|
///   it.list_arg.clone())`). Every position here is unconditionally a list.
///
/// Since a single `(command, args, arg_index)` triple can't tell which
/// encoding produced it, and *every* position is a genuine list position in
/// the inlined encoding while all but the last are in the opaque one, "every
/// in-range position" is the rule that is correct for one encoding and
/// correct-except-for-a-practically-unreachable-slot for the other.
fn loop_list_header_shimmer_type(
    spec: &tcl_registry::CommandSpec,
    args: &[&str],
    arg_index: usize,
) -> Option<TclType> {
    if !spec.traits.contains(Traits::LOOP_LIST_HEADER) {
        return None;
    }
    (arg_index < args.len()).then_some(TclType::List)
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
    fn arg_shimmer_type_foreach_opaque_shape_list_and_varlist_positions() {
        // Opaque (non-inlined) call shape: literal source args `varList list
        // body`. Both the varList (arg 0) and list (arg 1) positions are
        // List-shimmering — `Tcl_ForeachObjCmd` list-splits both — the body
        // (arg 2) is out of range once excluded... but since this rule
        // covers every in-range position (see the function's doc comment),
        // arg 2 also reports List here; `check_invocation` never reaches it
        // in practice since a real body is never a pure `$var` reference.
        let r = registry();
        let args = ["x", "$l", "body"];
        assert_eq!(
            arg_shimmer_type(&r, "foreach", &args, 0),
            Some(TclType::List)
        );
        assert_eq!(
            arg_shimmer_type(&r, "foreach", &args, 1),
            Some(TclType::List)
        );
    }

    #[test]
    fn arg_shimmer_type_foreach_inlined_shape_single_list_is_list() {
        // Inlined call shape (the default): the synthetic `var_def` Call's
        // `args` is list-only, one entry per iterator — `foreach x $l {...}`
        // becomes `args = ["$l"]`, arg 0.
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "foreach", &["$l"], 0),
            Some(TclType::List)
        );
    }

    #[test]
    fn arg_shimmer_type_foreach_inlined_shape_multi_iterator_all_list() {
        // `foreach a $la b $lb {...}` inlined: `args = ["$la", "$lb"]` — both
        // positions are list positions.
        let r = registry();
        let args = ["$la", "$lb"];
        assert_eq!(
            arg_shimmer_type(&r, "foreach", &args, 0),
            Some(TclType::List)
        );
        assert_eq!(
            arg_shimmer_type(&r, "foreach", &args, 1),
            Some(TclType::List)
        );
    }

    #[test]
    fn arg_shimmer_type_lmap_single_list_is_list() {
        let r = registry();
        assert_eq!(
            arg_shimmer_type(&r, "lmap", &["$l"], 0),
            Some(TclType::List)
        );
    }

    #[test]
    fn arg_shimmer_type_foreach_out_of_range_index_is_none() {
        let r = registry();
        assert_eq!(arg_shimmer_type(&r, "foreach", &["$l"], 1), None);
        assert_eq!(arg_shimmer_type(&r, "foreach", &[], 0), None);
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
