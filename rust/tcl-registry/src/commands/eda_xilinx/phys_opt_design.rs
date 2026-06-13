//! `phys_opt_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[
    FormSpec { kind: FormKind::Default, synopsis: "phys_opt_design ?-directive directive? ?-fanout_opt? ?-placement_opt? ?-rewire? ?-critical_cell_opt? ?-dsp_register_opt? ?-bram_register_opt? ?-hold_fix?" },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "phys_opt_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Run physical optimization after placement.", &["phys_opt_design ?-directive directive? ?-fanout_opt? ?-placement_opt? ?-rewire? ?-critical_cell_opt?"], "F5")),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
