//! `read_edif` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "read_edif",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Read an EDIF netlist file.",
            &["read_edif file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
