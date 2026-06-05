//! `html::tagstrip` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "html::tagstrip string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "html::tagstrip",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Remove all HTML tags from a string.",
            synopsis: &["html::tagstrip string"],
            snippet: "",
            source: "tcllib html package",
            examples: "",
            return_value: "The text with all HTML tags removed.",
        }),
        forms: FORMS,
        tcllib_package: Some("html"),
        required_package: Some("html"),
        ..CommandSpec::DEFAULT
    }
}
