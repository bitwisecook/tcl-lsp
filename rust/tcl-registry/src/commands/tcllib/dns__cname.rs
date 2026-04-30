//! `dns::cname` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::cname",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the canonical name from a DNS query result.",
            &["dns::cname token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
