//! `logger::levels` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::levels",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return the list of valid log levels.",
            &["logger::levels"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
