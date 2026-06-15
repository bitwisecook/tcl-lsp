//! `DHCPv6::peer_address` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::peer_address",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns peer address field from DHCPv6 RELAY message.",
            synopsis: &["DHCPv6::peer_address"],
            snippet: "This command returns peer address field from DHCPv6 RELAY message\n\nDetails (syntax):\nDHCPv6::peer_address",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__peer_address.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Peer_address [DHCPv6::peer_address]\"\n    }",
            return_value: "This command returns peer address field from DHCPv6 RELAY message",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv6::peer_address",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
