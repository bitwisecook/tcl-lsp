//! `textutil::chop` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::chop",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Remove the last character.",
            &["textutil::chop string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
