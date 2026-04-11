//! `DIAMETER::is_retransmission` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::is_retransmission",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns true if it is a retransmitted request, otherwise, returns false.",
            &["DIAMETER::is_retransmission"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
