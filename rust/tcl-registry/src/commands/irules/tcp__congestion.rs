//! `TCP::congestion` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::congestion",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the TCP congestion control algorithm.",
            &["TCP::congestion (none |"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
