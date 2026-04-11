//! `mime::getheader` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getheader",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Get header value(s) from a MIME token.",
            &["mime::getheader token ?key?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
