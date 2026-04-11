//! `DHCPv6::msg_type` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::msg_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns message type field from DHCPv6 message.",
            &["DHCPv6::msg_type"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
