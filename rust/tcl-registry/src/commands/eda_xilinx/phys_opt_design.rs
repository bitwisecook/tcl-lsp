//! `phys_opt_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "phys_opt_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Run physical optimization after placement.", &["phys_opt_design ?-directive directive? ?-fanout_opt? ?-placement_opt? ?-rewire? ?-critical_cell_opt?"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
