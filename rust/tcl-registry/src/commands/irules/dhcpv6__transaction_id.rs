//! `DHCPv6::transaction_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::transaction_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command returns transaction id field from DHCPv6 message.",
            &["DHCPv6::transaction_id"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}
