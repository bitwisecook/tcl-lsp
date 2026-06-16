//! `csv::joinlist` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "csv::joinlist values ?sepChar? ?quoteChar? ?quoteStyle?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::joinlist",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet {
            summary: "Join a list of lists into CSV-formatted lines.",
            synopsis: &["csv::joinlist values ?sepChar? ?quoteChar? ?quoteStyle?"],
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
