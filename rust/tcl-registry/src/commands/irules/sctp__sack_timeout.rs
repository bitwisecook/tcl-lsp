//! `SCTP::sack_timeout` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::sack_timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the SCTP's delayed selective acknowledgement timeout.",
            &["SCTP::sack_timeout (clientside | serverside)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
