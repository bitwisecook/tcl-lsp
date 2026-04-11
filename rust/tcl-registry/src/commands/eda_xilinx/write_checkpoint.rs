//! `write_checkpoint` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_checkpoint",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Save a design checkpoint (.dcp).",
            &["write_checkpoint ?-force? ?-cell cell? file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
