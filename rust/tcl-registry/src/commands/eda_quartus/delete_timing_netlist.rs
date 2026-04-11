//! `delete_timing_netlist` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "delete_timing_netlist",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Delete the current timing netlist.",
            &["delete_timing_netlist"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
