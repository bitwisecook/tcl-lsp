//! `initialize_floorplan` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "initialize_floorplan",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Initialize the floorplan from constraints.",
            &["initialize_floorplan ?-core_utilization util? ?-core_offset offset?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
