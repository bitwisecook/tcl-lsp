//! `TCP::mss` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::mss",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the Maximum Segment Size (MSS) for a TCP connection.",
            &["TCP::mss"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
