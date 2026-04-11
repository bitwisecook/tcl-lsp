//! `find` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "find",
        dialects: Some(DialectSet::MENTOR),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Find signals matching a pattern.",
            &["find ?-recursive? ?-type type? ?-ports? ?-signals? ?-internal? pattern"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
