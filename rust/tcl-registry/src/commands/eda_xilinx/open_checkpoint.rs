//! `open_checkpoint` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open_checkpoint",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Open a design checkpoint for editing.",
            &["open_checkpoint file_name"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
