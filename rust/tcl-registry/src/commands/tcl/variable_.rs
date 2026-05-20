//! `variable` — create and initialise a namespace variable.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Command spec for `variable`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "variable",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Create and initialise a namespace variable.",
            &["variable name", "variable ?name value...?"],
            "Tcl variable(1)",
        )),
        lowering_hook: Some(LoweringHookId::Variable),
        ..CommandSpec::DEFAULT
    }
}
