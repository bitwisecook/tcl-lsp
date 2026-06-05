//! `logger::levels` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::levels",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::levels",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Return the list of valid log levels.",
            synopsis: &["logger::levels"],
            snippet: "",
            source: "tcllib logger package",
            examples: "",
            return_value: "The list: debug info notice warn error critical alert emergency.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
