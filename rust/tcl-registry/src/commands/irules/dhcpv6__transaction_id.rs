//! `DHCPv6::transaction_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::transaction_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns transaction id field from DHCPv6 message.",
            synopsis: &["DHCPv6::transaction_id"],
            snippet: "This command returns transaction id field from DHCPv6 message\n\nDetails (syntax):\nDHCPv6::transaction_id",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__transation_id.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Transaction_id [DHCPv6::transaction_id]\"\n    }",
            return_value: "This command returns transaction id field from DHCPv6 message",
        }),
        ..CommandSpec::DEFAULT
    }
}
