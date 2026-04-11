//! `textutil::blank` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::blank",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return a string of N spaces.",
            &["textutil::blank n"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
