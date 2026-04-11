//! `csv::split2queue` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::split2queue",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 4),
        hover: Some(HoverSnippet::brief(
            "Split CSV data and store it into a queue object.",
            &["csv::split2queue ?-alternate? q line ?sepChar?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
