//! `mime::word_encode` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::word_encode charset method string ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::word_encode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(3),
        hover: Some(HoverSnippet {
            summary: "Encode a string per RFC 2047.",
            synopsis: &["mime::word_encode charset method string ?options?"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("mime"),
        required_package: Some("mime"),
        ..CommandSpec::DEFAULT
    }
}
