//! `DHCPv4::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command drops DHCPv4 message silently.",
            &["DHCPv4::drop"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
