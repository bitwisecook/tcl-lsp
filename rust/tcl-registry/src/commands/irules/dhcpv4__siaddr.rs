//! `DHCPv4::siaddr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::siaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns siaddr(server IP) field from DHCPv4 message.",
            synopsis: &["DHCPv4::siaddr"],
            snippet: "This command returns siaddr(server IP) field from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::siaddr",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__siaddr.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Siaddr [DHCPv4::siaddr]\"\n    }",
            return_value: "This command returns siaddr(server IP) field from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::siaddr" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
