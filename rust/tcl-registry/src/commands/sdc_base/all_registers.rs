//! `all_registers` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "all_registers",
        dialects: Some(DialectSet::SYNOPSYS | DialectSet::CADENCE | DialectSet::XILINX | DialectSet::QUARTUS | DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Return all register cells.", &["all_registers ?-clock clock_name? ?-rise_clock clock_name? ?-fall_clock clock_name? ?-cells? ?-data_"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
