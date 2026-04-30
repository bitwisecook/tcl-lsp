//! `lgen` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lgen",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Lazy list generator command (9.0+).",
            &["lgen"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
