//! `get_projects` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_projects",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get all open projects.",
            &["get_projects ?-regexp? ?-nocase? ?-filter expr? ?patterns?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
