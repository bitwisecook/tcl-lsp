//! `csv::report` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::report cmd matrix ?chan?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::report",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet {
            summary: "Format a matrix as a human-readable report.",
            synopsis: &["csv::report cmd matrix ?chan?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
