//! `logger::services` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::services",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return a list of all active logger services.",
            &["logger::services"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
