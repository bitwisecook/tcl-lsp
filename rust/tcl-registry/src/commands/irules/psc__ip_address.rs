//! `PSC::ip_address` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "PSC::ip_address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Get/set/remove ip address(es).",
            &["PSC::ip_address (IP_ADDR)*"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
