//! `json::dict2json` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "json::dict2json dictValue",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "json::dict2json",
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Convert a Tcl dict to a JSON string.",
            synopsis: &["json::dict2json dictValue"],
            snippet: "Converts a Tcl dictionary to a JSON-encoded string.",
            source: "tcllib json package",
            examples: "set json [json::dict2json [dict create name \"test\" value 42]]",
            return_value: "A JSON-encoded string.",
        }),
        forms: FORMS,
        tcllib_package: Some("json"),
        required_package: Some("json"),
        ..CommandSpec::DEFAULT
    }
}
