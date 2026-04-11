//! `get_db` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_db",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get a database attribute value.",
            &["get_db object_or_attr ?-regexp? ?-foreach script?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
