//! `TCP::client_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::client_port",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the client port of the TCP connection.",
            &["TCP::client_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
