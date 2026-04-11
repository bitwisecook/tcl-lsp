//! `resume` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "resume",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Resume simulation from a breakpoint.",
            &["resume"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
