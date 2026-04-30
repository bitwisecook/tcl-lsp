//! `dns::dump` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "dns::dump",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 2),
        hover: Some(HoverSnippet::brief(
            "Dump the contents of a DNS query token for debugging.",
            &["dns::dump token ?channel?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}
