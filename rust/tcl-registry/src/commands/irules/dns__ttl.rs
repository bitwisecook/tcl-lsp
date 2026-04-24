//! `DNS::ttl` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::ttl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the resource record TTL field.",
            &["DNS::ttl RR_OBJECT (TTL)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
