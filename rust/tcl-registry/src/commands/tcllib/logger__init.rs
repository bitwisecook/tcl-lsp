//! `logger::init` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::init",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Initialise a logger for a service.",
            &["logger::init service"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
