//! `write_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a design database.",
            &["write_design ?-innovus? ?-base_name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
