//! `textutil::trimleft` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::trimleft text ?regexp?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trimleft",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet {
            summary: "Trim leading characters from each line.",
            synopsis: &["textutil::trimleft text ?regexp?"],
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
