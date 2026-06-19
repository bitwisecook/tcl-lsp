//! `csv::join` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::join values ?sepChar? ?quoteChar?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::join",
        dialects: None,
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet {
            summary: "Join a list of values into a CSV-formatted line.",
            synopsis: &["csv::join values ?sepChar? ?quoteChar?"],
            snippet: "",
            source: "tcllib csv package",
            examples: "set line [csv::join $fields \",\"]",
            return_value: "A CSV-formatted string.",
        }),
        forms: FORMS,
        tcllib_package: Some("csv"),
        required_package: Some("csv"),
        ..CommandSpec::DEFAULT
    }
}
