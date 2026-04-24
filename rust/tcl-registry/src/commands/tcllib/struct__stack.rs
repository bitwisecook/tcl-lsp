//! `struct::stack` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::stack",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Create and manipulate LIFO stack objects.",
            &["struct::stack ?stackName?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
