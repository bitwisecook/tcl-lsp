//! `bc` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bc",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Clear (disable) breakpoints.",
            &["bc ?breakpoint_id | -all?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
