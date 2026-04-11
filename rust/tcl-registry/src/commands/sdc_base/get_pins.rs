//! `get_pins` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_pins",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Get pin objects matching a pattern.", &["get_pins ?-hierarchical? ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?-leaf? ?patterns?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
