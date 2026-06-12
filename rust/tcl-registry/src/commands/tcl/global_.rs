//! `global` — access global variables.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "global ?varname ...?",
}];

/// Command spec for `global`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "global",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::CREATES_BARRIER
            | Traits::CREATES_SCOPE_ALIAS
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::FRAME_HASH_BUILTIN,
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
            summary: "Access global variables",
            synopsis: &["global ?varname ...?"],
            snippet: "This command has no effect unless executed in the context of a proc body.",
            source: "Tcl man page global.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::Global),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
