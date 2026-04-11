//! `platform::identify` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "platform::identify",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(0),
        hover: Some(HoverSnippet::brief(
            "Return the platform identifier for the current machine.",
            &["platform::identify"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
