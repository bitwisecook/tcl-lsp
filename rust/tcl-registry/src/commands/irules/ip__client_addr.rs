//! `IP::client_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::client_addr",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the client IP address of a connection.",
            &["IP::client_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
