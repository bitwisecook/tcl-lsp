//! `ip::contract` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::contract",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Contract an IPv6 address to its shortest form.",
            &["ip::contract address"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
