//! `platform::patterns` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::patterns",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return a list of platform patterns that match the given identifier.",
            &["platform::patterns id"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
