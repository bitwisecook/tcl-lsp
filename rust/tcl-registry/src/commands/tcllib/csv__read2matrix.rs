//! `csv::read2matrix` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::read2matrix",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 5),
        hover: Some(HoverSnippet::brief(
            "Read CSV data from a channel into a matrix object.",
            &["csv::read2matrix ?-alternate? chan m ?sepChar? ?expand?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
