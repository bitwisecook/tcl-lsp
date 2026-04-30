//! `textutil::cap` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "textutil::cap",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Capitalise the first character.",
            &["textutil::cap string"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
