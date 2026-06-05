//! `unset` — delete variables.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unset ?-nocomplain? ?--? ?name name name ...?",
}];

/// Command spec for `unset`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unset",
        traits: Traits::FRAMELESS_RUNTIME | Traits::BYTE_COMPILED | Traits::DESTROYS_VARIABLE,
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
        hover: Some(HoverSnippet {
            summary: "Delete variables",
            synopsis: &["unset ?-nocomplain? ?--? ?name name name ...?"],
            snippet: "This command removes one or more variables.",
            source: "Tcl man page unset.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Unset),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
