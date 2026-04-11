//! `TCP::rto` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rto",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the current value of Retransmission timeout.",
            &["TCP::rto"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
