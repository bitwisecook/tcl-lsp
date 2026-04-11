//! `SCTP::rto_min` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::rto_min",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the minimum value of SCTP retransmission timeout.",
            &["SCTP::rto_min (clientside | serverside)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
