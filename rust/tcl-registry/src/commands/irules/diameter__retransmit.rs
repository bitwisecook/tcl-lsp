//! `DIAMETER::retransmit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Triggers the request associated to the current answer message for retransmission",
            &["DIAMETER::retransmit (disabled | busy | unable | retransmit) (NOTE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
