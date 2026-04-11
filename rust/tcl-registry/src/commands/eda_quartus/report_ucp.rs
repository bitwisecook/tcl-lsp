//! `report_ucp` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_ucp",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report unconstrained paths.",
            &["report_ucp ?-file file? ?-panel_name name?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
