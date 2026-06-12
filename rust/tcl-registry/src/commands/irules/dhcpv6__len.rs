//! `DHCPv6::len` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::len",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command returns the length of the DHCP packet length.",
            synopsis: &["DHCPv6::len"],
            snippet: "This command returns the length of the DHCP packet length\n\nDetails (syntax):\nDHCPv6::len",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__len.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Len [DHCPv6::len]\"\n    }",
            return_value: "This command returns the length of the DHCP packet length",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DHCPv6::len" },
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
