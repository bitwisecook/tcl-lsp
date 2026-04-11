//! `textutil::untabify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::untabify",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Convert tabs to spaces.",
            &["textutil::untabify string ?num?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
