//! `DHCPv4::secs` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::secs",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns xid(transaction ID) field from DHCPv4 message.",
            &["DHCPv4::secs"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
