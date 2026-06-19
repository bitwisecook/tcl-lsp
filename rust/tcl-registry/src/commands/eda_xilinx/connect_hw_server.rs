//! `connect_hw_server` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "connect_hw_server ?-url url? ?-allow_non_jtag?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "connect_hw_server",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Connect to a hardware server.",
            &["connect_hw_server ?-url url? ?-allow_non_jtag?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
