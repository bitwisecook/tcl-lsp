//! `validate_bd_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "validate_bd_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Validate the block design.",
            &["validate_bd_design ?-force?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
