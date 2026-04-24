//! `dns::result` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::result",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Return the result of a DNS query.",
            &["dns::result token ?options?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
