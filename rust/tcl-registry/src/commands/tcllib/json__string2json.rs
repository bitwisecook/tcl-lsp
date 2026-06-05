//! `json::string2json` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "json::string2json string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::string2json",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl string to a JSON string value.",
            synopsis: &["json::string2json string"],
            snippet: "",
            source: "tcllib json package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
