//! `mime::getTransferEncoding` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::getTransferEncoding token",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getTransferEncoding",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the transfer encoding of a MIME token.",
            synopsis: &["mime::getTransferEncoding token"],
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
