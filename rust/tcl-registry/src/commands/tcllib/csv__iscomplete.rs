//! `csv::iscomplete` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "csv::iscomplete",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Test whether a CSV record is complete or has unbalanced quotes.",
            &["csv::iscomplete data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
