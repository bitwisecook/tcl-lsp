//! `logger::disable` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::disable",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Disable logging at the specified level.",
            &["logger::disable level"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
