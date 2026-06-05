//! `append` — append to variable.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "append varName ?value value value ...?",
}];

/// Command spec for `append`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "append",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::BYTE_COMPILED
            | Traits::READS_BEFORE_WRITE
            | Traits::STRING_LIST_CONFUSION,
        arity: Arity::at_least(1),
        arg_roles: &[(0, ArgRole::VarWrite)],
        assigns_variable_at: Some(0),
        safe_on_uninit: Some(DialectSet::ALL_TCL),
        return_type: Some(TclType::String),
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::String),
                shimmers: true,
            },
        )],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Append to variable",
            synopsis: &["append varName ?value value value ...?"],
            snippet: "Append all of the value arguments to the current value of variable varName.",
            source: "Tcl man page append.n",
            examples: "",
            return_value: "",
        }),
        lowering_hook: Some(LoweringHookId::AppendOrLappend),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
