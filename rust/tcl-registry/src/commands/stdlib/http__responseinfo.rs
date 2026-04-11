//! `http::responseInfo` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::responseInfo",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        return_type: Some(TclType::Dict),
        hover: Some(HoverSnippet::brief(
            "Return a dict of response metadata.",
            &["http::responseInfo token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
