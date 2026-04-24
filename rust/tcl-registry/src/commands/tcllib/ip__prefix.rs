//! `ip::prefix` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::prefix",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the network prefix of an address/mask.",
            &["ip::prefix address/mask"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
