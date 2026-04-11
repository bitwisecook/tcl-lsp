//! `logger::walk` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::walk",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Walk the logger tree applying a command to each service.",
            &["logger::walk service command"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
