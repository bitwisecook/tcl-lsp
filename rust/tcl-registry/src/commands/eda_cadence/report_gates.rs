//! `report_gates` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_gates",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report gate-level statistics.",
            &["report_gates ?-power?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
