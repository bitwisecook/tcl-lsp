//! `get_report_panel_data` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "get_report_panel_data",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get data from a specific report panel cell.",
            &["get_report_panel_data -name panel_name -row row -col col"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
