//! `textutil::trimright` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::trimright text ?regexp?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trimright",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Trim trailing characters from each line.",
            synopsis: &["textutil::trimright text ?regexp?"],
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
