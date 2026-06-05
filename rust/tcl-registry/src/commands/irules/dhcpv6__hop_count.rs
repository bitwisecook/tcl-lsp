//! `DHCPv6::hop_count` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::hop_count",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns hop-count field from DHCPv6 relay message.",
            synopsis: &["DHCPv6::hop_count"],
            snippet: "This command returns hop-count field from DHCPv6 relay message\n\nDetails (syntax):\nDHCPv6::hop_count",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__hop_count.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Hop-count [DHCPv6::hop_count]\"\n    }",
            return_value: "This command returns hop-count field from DHCPv6 relay message",
        }),
        ..CommandSpec::DEFAULT
    }
}
