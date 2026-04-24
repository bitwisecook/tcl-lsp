//! `DNS::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the service state to enabled for the current DNS packet.",
            &["DNS::enable (DNS_COMPONENT)+"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
