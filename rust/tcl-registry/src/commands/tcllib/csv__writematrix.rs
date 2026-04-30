//! `csv::writematrix` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::writematrix",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet::brief(
            "Write a matrix object to a channel in CSV format.",
            &["csv::writematrix m chan ?sepChar? ?quoteChar?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
