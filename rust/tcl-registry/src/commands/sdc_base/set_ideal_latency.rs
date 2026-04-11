//! `set_ideal_latency` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_ideal_latency",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set ideal latency on a clock network.",
            &["set_ideal_latency ?-rise | -fall? ?-min | -max? delay object_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
