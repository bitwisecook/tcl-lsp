//! `TCP::snd_cwnd` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_cwnd",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the TCP congestion window (cwnd).",
            &["TCP::snd_cwnd"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
