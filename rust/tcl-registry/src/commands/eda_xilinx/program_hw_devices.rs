//! `program_hw_devices` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "program_hw_devices",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Program FPGA devices.",
            &["program_hw_devices ?device_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
