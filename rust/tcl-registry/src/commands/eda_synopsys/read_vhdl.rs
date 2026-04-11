//! `read_vhdl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_vhdl",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read VHDL source files.",
            &["read_vhdl ?-library lib? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
