//! `logger::initNamespace` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "logger::initNamespace",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Initialise logging for a namespace.",
            &["logger::initNamespace ns ?level?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
