//! `set_clock_gating_style` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_clock_gating_style",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Specify clock gating implementation style.", &["set_clock_gating_style ?-sequential_cell cell_type? ?-positive_edge_logic gate_type? ?-negative_edge"], "F5")),
        ..CommandSpec::DEFAULT
    }
}
