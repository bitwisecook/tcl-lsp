//! `open_hw_target` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_hw_target",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Open a hardware target (JTAG chain).",
            &["open_hw_target ?target_name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
