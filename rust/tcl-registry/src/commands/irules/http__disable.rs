//! `HTTP::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Changes the HTTP filter from full parsing to passthrough mode.",
            &["HTTP::disable (discard)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
