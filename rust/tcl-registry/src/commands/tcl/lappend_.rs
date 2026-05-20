//! `lappend` — append list elements onto a variable.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Command spec for `lappend`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lappend",
        traits: Traits::NOT_PROC_FACTORY | Traits::BYTE_COMPILED | Traits::READS_BEFORE_WRITE,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Append list elements onto a variable.",
            &["lappend varName ?value value value ...?"],
            "Tcl lappend(1)",
        )),
        lowering_hook: Some(LoweringHookId::AppendOrLappend),
        ..CommandSpec::DEFAULT
    }
}
