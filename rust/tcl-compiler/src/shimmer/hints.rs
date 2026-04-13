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
    let spec = registry.get(command)?;

    // Subcommand dispatch: spec has subcommands and args[0] names one.
    if !spec.subcommands.is_empty() {
        let sub_name = args.first().copied()?;
        let sub = spec.subcommand(sub_name)?;
        // arg_index 0 = subcommand word; subtract 1 for sub-relative index.
        let sub_idx = arg_index.checked_sub(1)? as u8;
        return sub
            .arg_types
            .iter()
            .find(|(i, _)| *i == sub_idx)
            .and_then(|(_, h)| if h.shimmers { h.expected } else { None });
    }

    spec.arg_types
        .iter()
        .find(|(i, _)| *i == arg_index as u8)
        .and_then(|(_, h)| if h.shimmers { h.expected } else { None })
}

/// Return `true` when the two types are numerically compatible — i.e. a
/// value of type `current` used in a context expecting `expected` would
/// not cause a shimmer.
///
/// Tcl's runtime treats `Int`, `Boolean`, and `Numeric` as interchangeable
/// in arithmetic and boolean contexts; no intrep conversion is needed.
#[must_use]
pub fn is_numeric_compatible(current: TclType, expected: TclType) -> bool {
    if current == expected {
        return true;
    }
    use TclType::{Boolean, Int, Numeric};
    matches!(
        (current, expected),
        (Int, Boolean)
            | (Boolean, Int)
            | (Int, Numeric)
            | (Numeric, Int)
            | (Boolean, Numeric)
            | (Numeric, Boolean)
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
