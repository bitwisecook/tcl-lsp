//! `client_addr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "client_addr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the client IP address of a connection.",
            &["client_addr"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
