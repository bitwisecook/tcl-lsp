//! `SCTP::client_port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::client_port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the SCTP port/service number of the specified client.",
            &["SCTP::client_port"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
