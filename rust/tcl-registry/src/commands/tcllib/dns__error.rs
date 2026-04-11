//! `dns::error` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::error",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the error message from a DNS query.",
            &["dns::error token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
