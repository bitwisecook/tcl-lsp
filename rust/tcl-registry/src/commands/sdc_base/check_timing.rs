//! `check_timing` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_timing",
        dialects: Some(
            DialectSet::SYNOPSYS
                | DialectSet::CADENCE
                | DialectSet::XILINX
                | DialectSet::QUARTUS
                | DialectSet::MENTOR,
        ),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check for unconstrained paths and issues.",
            &["check_timing ?-verbose? ?-override_defaults list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
