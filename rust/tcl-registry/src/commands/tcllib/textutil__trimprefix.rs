//! `textutil::trimPrefix` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trimPrefix",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Remove a prefix from a string.",
            &["textutil::trimPrefix text prefix"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
