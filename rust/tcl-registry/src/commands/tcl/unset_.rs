//! `unset` — delete variables.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

/// Command spec for `unset`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unset",
        traits: Traits::DESTROYS_VARIABLE,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        return_type: Some(TclType::String),
        options: &[
            OptionSpec {
                name: "-nocomplain",
                takes_value: false,
                value_hint: "",
                detail: "Suppress errors for non-existent variables.",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "End of options.",
                dialects: None,
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet::brief(
            "Delete variables.",
            &["unset ?-nocomplain? ?--? ?name name name ...?"],
            "Tcl unset(1)",
        )),
        lowering_hook: Some(LoweringHookId::Unset),
        ..CommandSpec::DEFAULT
    }
}
