//! `create_timing_netlist` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_timing_netlist",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create a timing netlist for analysis.",
            &["create_timing_netlist ?-model model? ?-post_map | -post_fit?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
