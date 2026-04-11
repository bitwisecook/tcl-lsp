//! `csv::join` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::join",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet::brief(
            "Join a list of values into a CSV-formatted line.",
            &["csv::join values ?sepChar? ?quoteChar?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
