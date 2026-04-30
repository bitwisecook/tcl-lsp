//! `mime::mapencoding` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::mapencoding",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Map a Tcl encoding name to a MIME charset.",
            &["mime::mapencoding encoding"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
