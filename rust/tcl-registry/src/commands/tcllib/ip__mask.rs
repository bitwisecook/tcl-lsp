//! `ip::mask` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::mask",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the network mask for an address.",
            &["ip::mask address"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
