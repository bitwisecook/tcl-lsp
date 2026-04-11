//! `textutil::tabify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::tabify",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Convert spaces to tabs.",
            &["textutil::tabify string ?num?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
