//! `textutil::untabify2` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::untabify2 string ?num?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::untabify2",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Convert tabs to spaces (position-aware).",
            synopsis: &["textutil::untabify2 string ?num?"],
            snippet: "",
            source: "tcllib textutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
