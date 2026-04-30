//! `DHCPv4::hlen` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::hlen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns hlen (hardware len) field from DHCPv4 message.",
            &["DHCPv4::hlen"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
