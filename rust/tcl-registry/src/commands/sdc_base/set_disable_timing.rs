//! `set_disable_timing` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_disable_timing ?-from from_pin? ?-to to_pin? object_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_disable_timing",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Disable timing arcs.",
            &["set_disable_timing ?-from from_pin? ?-to to_pin? object_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
