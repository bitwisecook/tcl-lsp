//! `dbShape` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dbShape",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Access shape data from the design database (legacy).",
            &["dbShape ?-shape type? ?-net net_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
