//! `DNS::type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Gets or sets the resource record type field.",
            &["DNS::type RR_OBJECT (DNS_TYPE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
