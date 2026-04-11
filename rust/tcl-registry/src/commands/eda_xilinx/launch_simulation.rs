//! `launch_simulation` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "launch_simulation",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Launch a simulation.",
            &["launch_simulation ?-mode mode? ?-scripts_only? ?-simset simset?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
