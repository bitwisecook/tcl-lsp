//! `DHCPv4::ciaddr` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::ciaddr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns ciaddr (client ip address) from DHCPv4 message.",
            synopsis: &["DHCPv4::ciaddr"],
            snippet: "This command returns ciaddr (client ip address) from DHCPv4 message\n\nDetails (syntax):\nDHCPv4::ciaddr",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__ciaddr.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Ciaddr [DHCPv4::ciaddr]\"\n    }",
            return_value: "This command returns ciaddr (client ip address) from DHCPv4 message",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv4::ciaddr" },
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
