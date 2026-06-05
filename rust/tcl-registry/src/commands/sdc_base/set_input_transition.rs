//! `set_input_transition` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_input_transition ?-rise | -fall? ?-min | -max? transition port_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_input_transition",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set input transition time on ports.",
            &["set_input_transition ?-rise | -fall? ?-min | -max? transition port_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
