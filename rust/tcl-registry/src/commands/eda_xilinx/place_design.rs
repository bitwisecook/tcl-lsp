//! `place_design` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "place_design ?-directive directive? ?-no_timing_driven? ?-unplace?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "place_design",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Run placement.",
            &["place_design ?-directive directive? ?-no_timing_driven? ?-unplace?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
