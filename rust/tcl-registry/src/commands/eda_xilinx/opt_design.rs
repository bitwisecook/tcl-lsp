//! `opt_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "opt_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Run logic optimization after synthesis.", &["opt_design ?-directive directive? ?-retarget? ?-propconst? ?-sweep? ?-bram_power_opt? ?-remap?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
