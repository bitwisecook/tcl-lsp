//! `dns::cleanup` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::cleanup",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet::brief(
            "Clean up resources associated with a DNS query.",
            &["dns::cleanup token"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
