//! `http::responseCode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseCode",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the numeric HTTP response code.",
            &["http::responseCode token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
