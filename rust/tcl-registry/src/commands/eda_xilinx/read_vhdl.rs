//! `read_vhdl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_vhdl",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Add VHDL source files to the project.",
            &["read_vhdl ?-library lib? ?-vhdl2008? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
