//! `get_nets` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_nets",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Get net objects matching a pattern.", &["get_nets ?-hierarchical? ?-regexp? ?-nocase? ?-filter expr? ?-of_objects objects? ?patterns?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
