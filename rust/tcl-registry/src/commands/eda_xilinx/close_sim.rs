//! `close_sim` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "close_sim",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Close the current simulation.",
            &["close_sim ?-force?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
