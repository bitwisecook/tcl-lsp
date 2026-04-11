//! `UDP::server_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::server_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the UDP port/service number of a server system.",
            &["UDP::server_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
