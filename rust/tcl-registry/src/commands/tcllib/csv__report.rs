//! `csv::report` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::report",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(2, 3),
        hover: Some(HoverSnippet::brief(
            "Format a matrix as a human-readable report.",
            &["csv::report cmd matrix ?chan?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
