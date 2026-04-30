//! `ip_ttl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip_ttl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Synonym for IP::ttl. Returns the TTL of the latest IP packet received.",
            &["ip_ttl"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
