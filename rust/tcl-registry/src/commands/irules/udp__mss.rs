//! `UDP::mss` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "UDP::mss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the on-wire Maximum Segment Size (MSS) for a UDP connection.",
            &["UDP::mss"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
