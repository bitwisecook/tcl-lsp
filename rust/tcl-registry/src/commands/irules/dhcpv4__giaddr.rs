//! `DHCPv4::giaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::giaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns giaddr (gateway or relay IP) field from DHCPv4 message.",
            synopsis: &["DHCPv4::giaddr"],
            snippet: "This command returns giaddr (gateway or relay IP) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4:: giaddr",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__giaddr.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Giaddr [DHCPv4::giaddr]\"\n    }",
            return_value: "This command returns giaddr (gateway or relay IP) field from DHCPv4 message",
        }),
        ..CommandSpec::DEFAULT
    }
}
