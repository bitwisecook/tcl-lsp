//! `variable` — create and initialise a namespace variable.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "variable name",
}];

/// Command spec for `variable`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "variable",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
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
hover: Some(HoverSnippet {
    summary: "create and initialize a namespace variable",
    synopsis: &["variable name", "variable ?name value...?"],
    snippet: "This command is normally used within a namespace eval command to create one or more variables within a namespace.",
    source: "Tcl man page variable.n",
    examples: "",
    return_value: "",
}),
        lowering_hook: Some(LoweringHookId::Variable),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
