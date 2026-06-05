//! `logger::services` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::services",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::services",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet {
            summary: "Return a list of all active logger services.",
            synopsis: &["logger::services"],
            snippet: "",
            source: "tcllib logger package",
            examples: "",
            return_value: "A list of service names.",
        }),
        forms: FORMS,
        tcllib_package: Some("logger"),
        required_package: Some("logger"),
        ..CommandSpec::DEFAULT
    }
}
