//! `textutil::uncap` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::uncap",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Lowercase the first character.",
            &["textutil::uncap string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
