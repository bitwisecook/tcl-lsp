//! `syn_map` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "syn_map",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Map the design to target technology.",
            &["syn_map ?-effort effort?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
