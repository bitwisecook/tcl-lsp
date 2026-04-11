//! `http::requestHeaders` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::requestHeaders",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Return the HTTP request headers as a list.",
            &["http::requestHeaders token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
