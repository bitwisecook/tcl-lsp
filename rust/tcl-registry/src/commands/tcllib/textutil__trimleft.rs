//! `textutil::trimleft` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::trimleft",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Trim leading characters from each line.",
            &["textutil::trimleft text ?regexp?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
