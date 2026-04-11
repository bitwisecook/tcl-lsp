//! `logger::enable` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::enable",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Enable logging at the specified level.",
            &["logger::enable level"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
