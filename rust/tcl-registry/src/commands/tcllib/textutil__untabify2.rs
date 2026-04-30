//! `textutil::untabify2` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::untabify2",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Convert tabs to spaces (position-aware).",
            &["textutil::untabify2 string ?num?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
