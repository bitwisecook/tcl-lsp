//! `be` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "be",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Enable breakpoints.",
            &["be ?breakpoint_id | -all?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
