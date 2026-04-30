//! `struct::set` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::set",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Set operations on Tcl lists (union, intersect, difference, etc.).",
            &["struct::set subcommand ?args ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
