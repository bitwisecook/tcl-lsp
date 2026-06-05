//! `csv::split2matrix` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::split2matrix ?-alternate? m line ?sepChar? ?quoteChar?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::split2matrix",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 5),
        hover: Some(HoverSnippet {
            summary: "Split CSV data and store it into a matrix object.",
            synopsis: &["csv::split2matrix ?-alternate? m line ?sepChar? ?quoteChar?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}
