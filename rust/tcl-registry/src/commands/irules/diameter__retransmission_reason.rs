//! `DIAMETER::retransmission_reason` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission_reason",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the reason for retransmitting the current retransmitted request.",
            &["DIAMETER::retransmission_reason"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
