//! `set_clock_latency` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_clock_latency ?-source? ?-early | -late? ?-rise | -fall? delay object_list",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_clock_latency",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set clock network latency.",
            &["set_clock_latency ?-source? ?-early | -late? ?-rise | -fall? delay object_list"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
