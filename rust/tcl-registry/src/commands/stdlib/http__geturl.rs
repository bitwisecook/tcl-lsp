//! `http::geturl` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::geturl",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Retrieve a URL — the primary command for the http package.",
            &["http::geturl url ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
