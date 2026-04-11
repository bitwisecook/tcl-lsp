//! `csv::joinlist` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::joinlist",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 4),
        hover: Some(HoverSnippet::brief(
            "Join a list of lists into CSV-formatted lines.",
            &["csv::joinlist values ?sepChar? ?quoteChar? ?quoteStyle?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
