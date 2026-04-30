//! `textutil::splitn` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::splitn",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Split a string into chunks of length N.",
            &["textutil::splitn str ?len?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
