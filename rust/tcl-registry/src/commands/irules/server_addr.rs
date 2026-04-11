//! `server_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "server_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the IP address of the server.",
            &["server_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
