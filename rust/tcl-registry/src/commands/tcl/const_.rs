//! `const` — define a constant variable (Tcl 9 / TIP 590).

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "const varName value",
}];

/// Command spec for `const`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "const",
        dialects: None,
        arity: Arity::new(2, 2),
        assigns_variable_at: Some(0),
        arg_roles: &[(0, ArgRole::VarWrite)],
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Define a constant variable.",
            synopsis: &["const varName value"],
            snippet: "",
            source: "Tcl const(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
