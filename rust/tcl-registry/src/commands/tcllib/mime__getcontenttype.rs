//! `mime::getContentType` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::getContentType",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the content type of a MIME token.",
            &["mime::getContentType token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
