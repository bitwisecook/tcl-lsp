//! `ip::normalize` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::normalize",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Normalise an IP address to its canonical form.",
            &["ip::normalize address"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
