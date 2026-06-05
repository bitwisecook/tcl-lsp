//! `DHCPv4::chaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::chaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns chaddr (client hardware address) from DHCPv4 message.",
            synopsis: &["DHCPv4::chaddr"],
            snippet: "This command returns chaddr (client hardware address) from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::chaddr",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__chaddr.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Chaddr [DHCPv4::chaddr]\"\n    }",
            return_value: "This command returns chaddr (client hardware address) from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::chaddr" },
        ],
        ..CommandSpec::DEFAULT
    }
}
