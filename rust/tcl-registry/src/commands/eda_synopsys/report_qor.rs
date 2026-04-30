//! `report_qor` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_qor",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report Quality of Results metrics.",
            &["report_qor ?-nosplit? ?-summary?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
