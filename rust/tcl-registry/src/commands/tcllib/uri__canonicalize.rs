//! `uri::canonicalize` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::canonicalize uri",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::canonicalize",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Canonicalize a URI.",
            synopsis: &["uri::canonicalize uri"],
            snippet: "",
            source: "tcllib uri package",
            examples: "",
            return_value: "The canonicalized URI string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
