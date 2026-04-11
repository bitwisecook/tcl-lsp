//! `dns::wait` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::wait",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Wait for a DNS query to complete.",
            &["dns::wait token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
