//! `set_clock_gating_style` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "set_clock_gating_style ?-sequential_cell cell_type? ?-positive_edge_logic gate_type? ?-negative_edge_logic gate_type? ?-control_point before|after? ?-control_signal scan_enable? ?-minimum_bitwidth n? ?-max_fanout n?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_clock_gating_style",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Specify clock gating implementation style.", &["set_clock_gating_style ?-sequential_cell cell_type? ?-positive_edge_logic gate_type? ?-negative_edge"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
