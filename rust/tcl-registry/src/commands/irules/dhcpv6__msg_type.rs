//! `DHCPv6::msg_type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv6::msg_type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns message type field from DHCPv6 message.",
            synopsis: &["DHCPv6::msg_type"],
            snippet: "This command returns message type field from DHCPv6 message\n\nDetails (syntax):\nDHCPv6::msg_type",
            source: "https://clouddocs.f5.com/api/irules/DHCPv6__msg_type.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Msg_type [DHCPv6::msg_type]\"\n    }",
            return_value: "This command returns message type field from DHCPv6 message",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv6::msg_type",
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
