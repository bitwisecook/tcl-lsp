//! `mime::getheader` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::getheader token ?key?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getheader",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Get header value(s) from a MIME token.",
            synopsis: &["mime::getheader token ?key?"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
