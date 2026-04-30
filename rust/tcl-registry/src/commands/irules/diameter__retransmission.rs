//! `DIAMETER::retransmission` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets of sets the current message's retransmission settings.",
            &["DIAMETER::retransmission action"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
