//! `read_verilog` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_verilog",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add Verilog source files to the project.",
            &["read_verilog ?-sv? ?-library lib? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
