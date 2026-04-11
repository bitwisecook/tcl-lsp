//! `set_case_analysis` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_case_analysis",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set constant case analysis on a port/pin.",
            &["set_case_analysis value port_pin_list"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
