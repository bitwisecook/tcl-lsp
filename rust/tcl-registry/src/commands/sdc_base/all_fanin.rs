//! `all_fanin` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "all_fanin",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Return all fanin of a pin/port.",
            &["all_fanin ?-to objects? ?-flat? ?-startpoints_only? ?-only_cells?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
