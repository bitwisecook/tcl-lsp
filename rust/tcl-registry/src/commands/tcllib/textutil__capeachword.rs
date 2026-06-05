//! `textutil::capEachWord` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::capEachWord sentence",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::capEachWord",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Capitalise each word.",
            synopsis: &["textutil::capEachWord sentence"],
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
