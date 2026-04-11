//! `mime::reversemapencoding` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::reversemapencoding",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Map a MIME charset to a Tcl encoding name.",
            &["mime::reversemapencoding mimeType"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
