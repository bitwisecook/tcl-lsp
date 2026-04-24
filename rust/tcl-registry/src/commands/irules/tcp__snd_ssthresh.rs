//! `TCP::snd_ssthresh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::snd_ssthresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the TCP slow start threshold (ssthresh).",
            &["TCP::snd_ssthresh"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
