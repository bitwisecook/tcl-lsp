//! `generate_target` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "generate_target",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Generate IP output products.",
            &["generate_target target_type ?-force? objects"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
