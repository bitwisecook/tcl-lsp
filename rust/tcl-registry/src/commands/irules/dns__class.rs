//! `DNS::class` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::class",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the resource record class field.",
            &["DNS::class RR_OBJECT (DNS_CLASS)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
