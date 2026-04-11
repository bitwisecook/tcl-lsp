//! `DIAMETER::retransmission_default` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission_default",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets of sets the current connection's retransmission settings.",
            &["DIAMETER::retransmission_default action"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
