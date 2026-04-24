//! `DHCPv4::hops` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::hops",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns hops (number of hops) field from DHCPv4 message.",
            &["DHCPv4::hops"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
