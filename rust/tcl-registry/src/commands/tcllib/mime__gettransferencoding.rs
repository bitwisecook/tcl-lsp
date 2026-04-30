//! `mime::getTransferEncoding` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getTransferEncoding",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the transfer encoding of a MIME token.",
            &["mime::getTransferEncoding token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
