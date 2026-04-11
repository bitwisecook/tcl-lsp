//! `csv::writequeue` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::writequeue",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet::brief(
            "Write a queue object to a channel in CSV format.",
            &["csv::writequeue q chan ?sepChar? ?quoteChar?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
