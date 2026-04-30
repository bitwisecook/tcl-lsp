//! `write_bitstream` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_bitstream",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Generate a bitstream file.",
            &["write_bitstream ?-force? ?-bin_file? ?-no_partial_bitfile? ?-cell cell? file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
