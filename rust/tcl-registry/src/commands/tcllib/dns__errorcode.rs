//! `dns::errorcode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::errorcode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Return the error code from a DNS query.",
            &["dns::errorcode token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
