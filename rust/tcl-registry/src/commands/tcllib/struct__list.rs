//! `struct::list` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "struct::list",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Advanced list manipulation commands.",
            &["struct::list subcommand ?args ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
