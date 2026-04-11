//! `csv::split2matrix` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::split2matrix",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 5),
        hover: Some(HoverSnippet::brief(
            "Split CSV data and store it into a matrix object.",
            &["csv::split2matrix ?-alternate? m line ?sepChar? ?quoteChar?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
