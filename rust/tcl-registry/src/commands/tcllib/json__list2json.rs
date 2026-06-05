//! `json::list2json` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "json::list2json list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::list2json",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl list to a JSON array.",
            synopsis: &["json::list2json list"],
            snippet: "",
            source: "tcllib json package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
