//! `textutil::blank` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "textutil::blank n",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::blank",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return a string of N spaces.",
            synopsis: &["textutil::blank n"],
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
