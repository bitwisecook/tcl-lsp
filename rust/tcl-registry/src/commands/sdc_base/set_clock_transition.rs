//! `set_clock_transition` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_clock_transition ?-rise | -fall? ?-min | -max? transition clock_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_clock_transition",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set clock transition time.",
            &["set_clock_transition ?-rise | -fall? ?-min | -max? transition clock_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
