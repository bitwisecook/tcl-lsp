//! `set_disable_timing` command.
use crate::prelude::*;
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
        ..CommandSpec::DEFAULT
    }
}
