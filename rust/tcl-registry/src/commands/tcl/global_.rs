//! `global` — access global variables.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Command spec for `global`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "global",
        traits: Traits::LANGUAGE_KEYWORD
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
            "Access global variables.",
            &["global ?varname ...?"],
            "Tcl global(1)",
        )),
        lowering_hook: Some(LoweringHookId::Global),
        ..CommandSpec::DEFAULT
    }
}
