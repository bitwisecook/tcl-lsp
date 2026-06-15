//! `compile_ultra` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "compile_ultra ?-incremental? ?-retime? ?-scan? ?-no_autoungroup? ?-no_boundary_optimization? ?-gate_clock? ?-timing_high_effort_script?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "compile_ultra",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Compile with advanced optimizations.",
            &[
                "compile_ultra ?-incremental? ?-retime? ?-scan? ?-no_autoungroup? ?-no_boundary_optimization? ?-gate_",
            ],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
