//! `SCTP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sends the specified data directly to the peer.",
            &["SCTP::respond (PAYLOAD_DATA (PAYLOAD_OFFSET)? (PAYLOAD_LENGTH)?)"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
