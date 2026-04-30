//! `read_netlist` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_netlist",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read a gate-level netlist.",
            &["read_netlist file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
