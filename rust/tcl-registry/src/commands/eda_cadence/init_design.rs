//! `init_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "init_design",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Initialize the design for implementation.",
            &["init_design"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
