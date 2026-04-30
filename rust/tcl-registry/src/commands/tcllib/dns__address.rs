//! `dns::address` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::address",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the IP addresses from a DNS query result.",
            &["dns::address token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
