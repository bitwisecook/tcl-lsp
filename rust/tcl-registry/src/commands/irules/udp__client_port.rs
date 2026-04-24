//! `UDP::client_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::client_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the UDP port/service number of a client system.",
            &["UDP::client_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
