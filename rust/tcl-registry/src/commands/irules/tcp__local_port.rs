//! `TCP::local_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::local_port",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the local port of a TCP connection.",
            &["TCP::local_port (clientside | serverside)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
