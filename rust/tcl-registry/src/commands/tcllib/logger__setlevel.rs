//! `logger::setlevel` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::setlevel",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Set the logging level for all logger services.",
            &["logger::setlevel level"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
