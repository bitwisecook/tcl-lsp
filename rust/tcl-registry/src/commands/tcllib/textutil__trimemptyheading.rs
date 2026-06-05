//! `textutil::trimEmptyHeading` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::trimEmptyHeading text",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trimEmptyHeading",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Remove leading empty lines.",
            synopsis: &["textutil::trimEmptyHeading text"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
