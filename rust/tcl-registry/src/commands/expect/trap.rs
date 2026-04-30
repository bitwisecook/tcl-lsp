//! `trap` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "trap",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Trap signals and execute a command when they occur.",
            &["trap ?command? ?signal ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
