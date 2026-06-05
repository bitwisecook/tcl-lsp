//! `logger::servicecmd` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "logger::servicecmd service",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::servicecmd",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Return the command token for a named logger service.",
            synopsis: &["logger::servicecmd service"],
            snippet: "",
            source: "tcllib logger package",
            examples: "",
            return_value: "The logger command for the named service.",
        }),
        forms: FORMS,
        tcllib_package: Some("logger"),
        required_package: Some("logger"),
        ..CommandSpec::DEFAULT
    }
}
