//! `DHCPv6::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command drops DHCPv6 message silently.",
            &["DHCPv6::drop"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
