//! `dns::status` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::status",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the status of a DNS query.",
            &["dns::status token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
