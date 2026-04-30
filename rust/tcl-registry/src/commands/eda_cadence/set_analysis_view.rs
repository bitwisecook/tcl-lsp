//! `set_analysis_view` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_analysis_view",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set active analysis views for setup and hold.",
            &["set_analysis_view ?-setup view_list? ?-hold view_list?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
