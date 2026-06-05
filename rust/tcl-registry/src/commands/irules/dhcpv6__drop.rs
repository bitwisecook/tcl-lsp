//! `DHCPv6::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command drops DHCPv6 message silently.",
            synopsis: &["DHCPv6::drop"],
            snippet:
                "This command drops DHCPv6 message silently\n\nDetails (syntax):\nDHCPv6::drop",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__drop.html",
            examples: "when CLIENT_DATA {\n        DHCPv6::drop\n    }",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
