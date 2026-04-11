//! `DHCPv6::hop_count` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::hop_count",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns hop-count field from DHCPv6 relay message.",
            &["DHCPv6::hop_count"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
