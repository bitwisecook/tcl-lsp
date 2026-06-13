//! `uri::join` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "uri::join ?key value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "uri::join",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Construct a URI from component parts.",
            synopsis: &["uri::join ?key value ...?"],
            snippet: "",
            source: "tcllib uri package",
            examples: "set url [uri::join scheme https host example.com path /index.html]",
            return_value: "A URI string.",
        }),
        forms: FORMS,
        tcllib_package: Some("uri"),
        required_package: Some("uri"),
        ..CommandSpec::DEFAULT
    }
}
