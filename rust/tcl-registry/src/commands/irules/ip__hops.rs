//! `IP::hops` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::hops",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gives you the estimated number of hops the peer takes to get to you.",
            &["IP::hops"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
