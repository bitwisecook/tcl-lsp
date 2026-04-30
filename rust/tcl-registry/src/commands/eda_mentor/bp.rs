//! `bp` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "bp",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Set a breakpoint.",
            &["bp ?file_name? ?line_number? ?-cond condition?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
