//! `set_ideal_network` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_ideal_network",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Mark a network as ideal.",
            &["set_ideal_network ?-no_propagate? object_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
