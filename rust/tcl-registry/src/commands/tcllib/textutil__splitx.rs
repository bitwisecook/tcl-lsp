//! `textutil::splitx` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::splitx",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Split a string using a regexp pattern as delimiter.",
            &["textutil::splitx string ?regexp?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
