//! `csv::split2queue` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::split2queue ?-alternate? q line ?sepChar?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::split2queue",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet {
            summary: "Split CSV data and store it into a queue object.",
            synopsis: &["csv::split2queue ?-alternate? q line ?sepChar?"],
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
