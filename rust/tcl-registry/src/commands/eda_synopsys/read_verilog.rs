//! `read_verilog` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_verilog",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Read Verilog source files.",
            &["read_verilog ?-define define_list? file_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
