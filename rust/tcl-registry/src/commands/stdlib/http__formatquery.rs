//! `http::formatQuery` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::formatQuery",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet::brief(
            "Generate an x-url-encoded query string from key/value pairs.",
            &["http::formatQuery key value ?key value ...?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
