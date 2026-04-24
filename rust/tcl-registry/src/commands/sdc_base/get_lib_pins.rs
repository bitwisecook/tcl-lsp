//! `get_lib_pins` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_lib_pins",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get library pin objects matching a pattern.",
            &["get_lib_pins ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?patterns?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
