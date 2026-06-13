//! `coroinject` — inject a command into a coroutine.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroinject coroName command ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroinject",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Inject a command into a suspended coroutine.",
            synopsis: &["coroinject coroName command ?arg ...?"],
            snippet: "",
            source: "Tcl coroinject(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
