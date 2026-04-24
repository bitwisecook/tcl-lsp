//! `ASM::client_ip` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::client_ip",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the IP address of the end client that sent the request.",
            &["ASM::client_ip"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
