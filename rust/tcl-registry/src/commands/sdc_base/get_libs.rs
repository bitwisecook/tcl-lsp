//! `get_libs` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_libs",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get library objects matching a pattern.",
            &["get_libs ?-regexp? ?-nocase? ?-filter expr? ?patterns?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
