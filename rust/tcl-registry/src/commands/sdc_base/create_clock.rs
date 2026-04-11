//! `create_clock` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_clock",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief("Create a clock object.", &["create_clock ?-period period? ?-name name? ?-waveform edge_list? ?-add? ?source_objects?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
