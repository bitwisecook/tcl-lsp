//! `textutil::undent` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::undent text",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::undent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Remove common leading whitespace from all lines.",
            synopsis: &["textutil::undent text"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "set clean [textutil::undent $text]",
            return_value: "The undented text.",
        }),
        forms: FORMS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}
