//! `textutil::tabify` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::tabify string ?num?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::tabify",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Convert spaces to tabs.",
            synopsis: &["textutil::tabify string ?num?"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("textutil"),
        required_package: Some("textutil"),
        ..CommandSpec::DEFAULT
    }
}
