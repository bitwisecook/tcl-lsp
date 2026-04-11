//! `dbGet` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dbGet",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get a design database object attribute (legacy).",
            &["dbGet object_spec.attribute ?-regexp pattern? ?-e?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
