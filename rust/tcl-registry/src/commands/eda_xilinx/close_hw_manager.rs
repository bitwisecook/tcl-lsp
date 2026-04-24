//! `close_hw_manager` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close_hw_manager",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Close the hardware manager.",
            &["close_hw_manager"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
