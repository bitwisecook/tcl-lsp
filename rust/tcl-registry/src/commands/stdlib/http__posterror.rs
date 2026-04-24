//! `http::postError` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::postError",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the post-request error message, if any.",
            &["http::postError token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
