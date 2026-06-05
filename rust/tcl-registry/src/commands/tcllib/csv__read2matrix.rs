//! `csv::read2matrix` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::read2matrix ?-alternate? chan m ?sepChar? ?expand?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::read2matrix",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 5),
        hover: Some(HoverSnippet {
            summary: "Read CSV data from a channel into a matrix object.",
            synopsis: &["csv::read2matrix ?-alternate? chan m ?sepChar? ?expand?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "",
            return_value: "The number of lines read.",
        }),
        forms: FORMS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}
