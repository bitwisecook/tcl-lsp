//! `dns::reset` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::reset",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet::brief(
            "Reset a DNS query.",
            &["dns::reset token ?reason? ?message?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
