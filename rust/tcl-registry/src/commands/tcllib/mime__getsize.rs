//! `mime::getsize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getsize",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the size of a MIME message.",
            &["mime::getsize token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
