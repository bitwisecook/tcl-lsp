//! `platform::generic` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::generic",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return the generic platform identifier (less specific than identify).",
            &["platform::generic"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
