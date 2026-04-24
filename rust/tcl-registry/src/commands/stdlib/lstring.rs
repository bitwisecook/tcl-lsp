//! `lstring` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lstring",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "String-backed list command for testing (9.0+).",
            &["lstring"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
