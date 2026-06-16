//! `coroprobe` — probe a suspended coroutine.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "coroprobe coroName command ?arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "coroprobe",
        dialects: Some(DialectSet::TCL90),
        arity: Arity::at_least(2),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Evaluate a command in a suspended coroutine.",
            synopsis: &["coroprobe coroName command ?arg ...?"],
            snippet: "",
            source: "Tcl coroprobe(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
