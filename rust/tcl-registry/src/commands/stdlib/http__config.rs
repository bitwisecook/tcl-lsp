//! `http::config` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::config",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get or set http package configuration options.",
            &["http::config"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
