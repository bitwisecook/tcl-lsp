//! `uri::resolve` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::resolve base url",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::resolve",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Resolve a relative URI reference against a base URI.",
            synopsis: &["uri::resolve base url"],
            snippet: "",
            source: "tcllib uri package",
            examples: "set abs [uri::resolve $baseUrl $relativeUrl]",
            return_value: "The resolved absolute URI.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
