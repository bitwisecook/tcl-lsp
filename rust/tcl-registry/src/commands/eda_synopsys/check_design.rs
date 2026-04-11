//! `check_design` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "check_design",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Check the design for consistency problems.",
            &["check_design ?-summary? ?-no_warnings?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
