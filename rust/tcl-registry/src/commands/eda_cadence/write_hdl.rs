//! `write_hdl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "write_hdl",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Write synthesized HDL netlist.",
            &["write_hdl ?-lec? ?design_name? > file"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
