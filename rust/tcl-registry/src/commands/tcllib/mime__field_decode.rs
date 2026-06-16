//! `mime::field_decode` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::field_decode field",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::field_decode",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Decode an RFC 2047 header field.",
            synopsis: &["mime::field_decode field"],
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
