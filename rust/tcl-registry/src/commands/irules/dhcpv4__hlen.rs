//! `DHCPv4::hlen` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::hlen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns hlen (hardware len) field from DHCPv4 message.",
            synopsis: &["DHCPv4::hlen"],
            snippet: "This command returns hlen (hardware len) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::hlen",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__hlen.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Hlen [DHCPv4::hlen]\"\n    }",
            return_value: "This command returns hlen (hardware len) field from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::hlen" },
        ],
        ..CommandSpec::DEFAULT
    }
}
