//! `ip::version` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::version",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the IP version of an address (4 or 6).",
            &["ip::version address"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
