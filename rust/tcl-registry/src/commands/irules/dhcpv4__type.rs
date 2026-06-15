//! `DHCPv4::type` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DHCPv4::type",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command returns type of DHCPv4 message.",
            synopsis: &["DHCPv4::type"],
            snippet: "This command returns type of DHCPv4 message\n\nDetails (syntax):\nDHCPv4::type",
            source: "https://clouddocs.f5.com/api/irules/DHCPv4__type.html",
            examples: "when CLIENT_DATA {\n        log local0. \"Type [DHCPv4::type]\"\n    }",
            return_value: "This command returns type of DHCPv4 message",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DHCPv4::type",
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
