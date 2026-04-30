//! `TCP::rttvar` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rttvar",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns TCP's smoothed RTT variance estimate.",
            &["TCP::rttvar"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
