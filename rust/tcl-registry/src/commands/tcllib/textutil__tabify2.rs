//! `textutil::tabify2` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::tabify2",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Convert spaces to tabs (position-aware).",
            &["textutil::tabify2 string ?num?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
