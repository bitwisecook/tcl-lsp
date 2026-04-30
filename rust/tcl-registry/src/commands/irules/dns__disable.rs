//! `DNS::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Sets the service state to disabled for the current DNS packet.",
            &["DNS::disable (DNS_COMPONENT)+"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
