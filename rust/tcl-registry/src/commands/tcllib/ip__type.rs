//! `ip::type` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::type",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the type of an IP address.",
            &["ip::type address"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
