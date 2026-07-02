//! `lpop` — get and remove an element from a list variable (Tcl 9.0+, TIP 323).

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lpop varName ?index ...?",
}];

/// Command spec for `lpop`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lpop",
        // `lpop` reads the list's current value before removing an element.
        traits: Traits::READS_BEFORE_WRITE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::List),
                shimmers: true,
            },
        )],
        return_type: Some(TclType::String),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Get and remove an element in a list variable.",
            synopsis: &["lpop varName ?index ...?"],
            snippet: "",
            source: "Tcl 9 man page lpop.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
