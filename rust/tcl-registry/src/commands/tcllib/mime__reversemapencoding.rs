//! `mime::reversemapencoding` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::reversemapencoding mimeType",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::reversemapencoding",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Map a MIME charset to a Tcl encoding name.",
            synopsis: &["mime::reversemapencoding mimeType"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
