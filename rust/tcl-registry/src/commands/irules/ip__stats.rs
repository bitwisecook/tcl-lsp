//! `IP::stats` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::stats",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Supplies information about the number of packets or bytes being sent or received",
            &["IP::stats ((pkts ('in' | 'out')?) | (bytes ('in' | 'out')?) | in | out | age)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
