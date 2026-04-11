//! `open_bd_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_bd_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Open a block design.",
            &["open_bd_design ?-name name? file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
