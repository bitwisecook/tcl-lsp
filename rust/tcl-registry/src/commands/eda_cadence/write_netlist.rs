//! `write_netlist` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_netlist",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write a gate-level netlist.",
            &["write_netlist file_name ?-top_module_first?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
