//! `dns::name` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::name",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the domain name from a DNS query result.",
            &["dns::name token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
