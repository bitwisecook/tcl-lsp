//! `save_bd_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "save_bd_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Save the current block design.",
            &["save_bd_design"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
