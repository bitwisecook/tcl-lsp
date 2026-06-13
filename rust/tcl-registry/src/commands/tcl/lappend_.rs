//! `lappend` — append list elements onto a variable.

use crate::hooks::LoweringHookId;
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lappend varName ?value value value ...?",
}];

/// Command spec for `lappend`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lappend",
        traits: Traits::FRAMELESS_RUNTIME
            | Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::READS_BEFORE_WRITE
            | Traits::FIRST_ARG_VARNAME,
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
hover: Some(HoverSnippet {
    summary: "Append list elements onto a variable",
    synopsis: &["lappend varName ?value value value ...?"],
    snippet: "This command treats the variable given by varName as a list and appends each of the value arguments to that list as a separate element, with spaces between elements.",
    source: "Tcl man page lappend.n",
    examples: "",
    return_value: "",
}),
        lowering_hook: Some(LoweringHookId::AppendOrLappend),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
