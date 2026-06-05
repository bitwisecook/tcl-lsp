//! `mime::word_decode` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::word_decode encoded",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::word_decode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Decode an RFC 2047 encoded word.",
            synopsis: &["mime::word_decode encoded"],
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
