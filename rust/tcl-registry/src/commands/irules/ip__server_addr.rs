//! `IP::server_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::server_addr",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the server's IP address.",
            &["IP::server_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
