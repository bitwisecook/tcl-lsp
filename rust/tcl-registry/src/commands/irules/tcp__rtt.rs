//! `TCP::rtt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rtt",
        traits: Traits::PURE | Traits::CSE_CANDIDATE,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the smoothed round-trip time estimate for a TCP connection.",
            &["TCP::rtt"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
