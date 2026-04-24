//! `NAME::response` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "NAME::response",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Deprecated: Returns a list of records received in response to a DNS query.",
            &["NAME::response"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
