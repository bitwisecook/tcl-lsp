//! `DHCPv4::yiaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::yiaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns yiaddr(your IP) field from DHCPv4 message.",
            synopsis: &["DHCPv4::yiaddr"],
            snippet: "This command returns yiaddr(your IP) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::yiaddr",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__yiaddr.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Yiaddr [DHCPv4::yiaddr]\"\n    }",
            return_value: "This command returns yiaddr(your IP) field from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::yiaddr" },
        ],
        ..CommandSpec::DEFAULT
    }
}
