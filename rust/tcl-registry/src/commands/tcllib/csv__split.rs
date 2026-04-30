//! `csv::split` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::split",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet::brief(
            "Split a CSV-formatted line into a list of values.",
            &["csv::split line ?sepChar? ?quoteChar?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
