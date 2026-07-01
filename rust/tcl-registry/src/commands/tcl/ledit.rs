//! `ledit` — replace elements in a list variable, in place (Tcl 9.0+, TIP 638).

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ledit listVar first last ?element element ...?",
}];

/// Command spec for `ledit`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ledit",
        // `ledit` reads the list variable's current value, replaces a range,
        // and writes the result back — a read-before-write of `listVar`.
        traits: Traits::READS_BEFORE_WRITE,
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(3),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Replace elements in a list variable, in place.",
            synopsis: &["ledit listVar first last ?element element ...?"],
            snippet: "ledit replaces zero or more elements (between indices first and last) of the list stored in listVar with the element arguments, updating the variable and returning its new value.",
            source: "Tcl 9 man page ledit.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_types: &[
            (
                0,
                ArgTypeHint {
                    expected: Some(TclType::List),
                    shimmers: true,
                },
            ),
            (
                1,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
            (
                2,
                ArgTypeHint {
                    expected: Some(TclType::Int),
                    shimmers: true,
                },
            ),
        ],
        ..CommandSpec::DEFAULT
    }
}
