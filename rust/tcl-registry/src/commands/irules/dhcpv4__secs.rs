//! `DHCPv4::secs` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::secs",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns xid(transaction ID) field from DHCPv4 message.",
            synopsis: &["DHCPv4::secs"],
            snippet: "This command returns xid(transaction ID) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::secs",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__secs.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Secs [DHCPv4::secs]\"\n    }",
            return_value: "This command returns xid(transaction ID) field from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::secs" },
        ],
        ..CommandSpec::DEFAULT
    }
}
