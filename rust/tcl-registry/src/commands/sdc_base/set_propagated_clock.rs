//! `set_propagated_clock` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_propagated_clock",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Specify that clock latency should be propagated.",
            &["set_propagated_clock object_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
