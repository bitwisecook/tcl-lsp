//! `base64::decode` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "base64::decode encodedData",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "base64::decode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Decode a base64-encoded string back to binary data.",
            synopsis: &["base64::decode encodedData"],
            snippet: "",
            source: "tcllib base64 package",
            examples: "set binary [base64::decode $encodedString]",
            return_value: "The decoded binary data.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
