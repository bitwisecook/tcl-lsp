//! `csv::joinmatrix` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::joinmatrix",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet::brief(
            "Join a matrix object into CSV-formatted lines.",
            &["csv::joinmatrix matrix ?sepChar? ?quoteChar? ?quoteStyle?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
