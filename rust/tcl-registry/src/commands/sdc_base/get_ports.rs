//! `get_ports` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_ports",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get port objects matching a pattern.",
            &["get_ports ?-regexp? ?-nocase? ?-filter expr? ?patterns?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
