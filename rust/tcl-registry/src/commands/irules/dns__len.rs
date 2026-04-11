//! `DNS::len` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::len",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the DNS packet message length.",
            &["DNS::len"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
