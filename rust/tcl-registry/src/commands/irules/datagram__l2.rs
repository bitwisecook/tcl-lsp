//! `DATAGRAM::l2` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DATAGRAM::l2",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns Layer 2 destination address.",
            &["DATAGRAM::l2 dest"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
