//! `textutil::indent` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::indent text prefix ?skip?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::indent",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Indent each line of text by a given prefix.",
            synopsis: &["textutil::indent text prefix ?skip?"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "set indented [textutil::indent $text \"    \"]",
            return_value: "The indented text.",
        }),
        forms: FORMS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}
