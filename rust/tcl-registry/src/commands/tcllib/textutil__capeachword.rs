//! `textutil::capEachWord` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::capEachWord",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Capitalise each word.",
            &["textutil::capEachWord sentence"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
