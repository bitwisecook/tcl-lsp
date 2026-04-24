//! `http::reset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::reset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Reset an HTTP transaction.",
            &["http::reset token ?why?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
