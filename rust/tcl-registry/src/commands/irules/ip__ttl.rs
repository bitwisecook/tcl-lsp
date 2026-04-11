//! `IP::ttl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IP::ttl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Returns the TTL of the latest IP packet received.",
            &["IP::ttl"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
