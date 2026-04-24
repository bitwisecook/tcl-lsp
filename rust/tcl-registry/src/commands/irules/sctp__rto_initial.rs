//! `SCTP::rto_initial` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::rto_initial",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the initial value of SCTP retransmission timeout.",
            &["SCTP::rto_initial (clientside | serverside)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
