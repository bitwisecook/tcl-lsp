//! `TCP::server_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::server_port",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the remote TCP port/service number of the serverside TCP connection.",
            &["TCP::server_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
