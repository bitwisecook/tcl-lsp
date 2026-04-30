//! `client_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "client_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the TCP port number/service of the specified client.",
            &["client_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
