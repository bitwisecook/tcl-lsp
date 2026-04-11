//! `save_report` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "save_report",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Save the compilation report.",
            &["save_report"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
